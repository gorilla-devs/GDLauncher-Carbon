use std::io::{self, Seek};

use crate::domain::instance::ModFileMetadata;
use anyhow::anyhow;
use serde::Deserialize;
use std::io::{BufRead, Read};

#[derive(Deserialize)]
#[serde(untagged)]
enum McModInfoContainer {
    Old(Vec<McModInfo>),
    New(NewMcModInfo),
}

#[derive(Deserialize)]
struct NewMcModInfo {
    #[serde(rename = "modList")]
    mod_list: Vec<NewMcModInfoObj>,
}

#[derive(Deserialize)]
struct NewMcModInfoObj {
    modid: String,
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
    #[serde(rename = "authorList")]
    authors: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct McModInfo {
    modid: String,
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
    authors: Option<Vec<String>>,
}

impl From<NewMcModInfoObj> for McModInfo {
    fn from(value: NewMcModInfoObj) -> Self {
        Self {
            modid: value.modid,
            name: value.name,
            version: value.version,
            description: value.description,
            authors: value.authors,
        }
    }
}

impl TryFrom<McModInfoContainer> for McModInfo {
    type Error = anyhow::Error;

    fn try_from(value: McModInfoContainer) -> Result<Self, Self::Error> {
        use McModInfoContainer as Container;

        let meta = match value {
            Container::Old(mod_list) => mod_list.into_iter().next().map(Into::into),
            Container::New(NewMcModInfo { mod_list }) => {
                mod_list.into_iter().next().map(Into::into)
            }
        };

        meta.ok_or_else(|| anyhow!("mcmod.info contained no mod entries"))
    }
}

#[derive(Deserialize)]
struct ModsToml {
    mods: Vec<ModsTomlEntry>,
}

#[derive(Deserialize)]
struct ModsTomlEntry {
    #[serde(rename = "modId")]
    modid: String,
    version: String,
    #[serde(rename = "displayName")]
    display_name: String,
    description: Option<String>,
    authors: Option<String>,
}

impl From<McModInfo> for ModFileMetadata {
    fn from(value: McModInfo) -> Self {
        Self {
            modid: value.modid,
            name: value.name,
            version: value.version,
            description: value.description,
            authors: value.authors.map(|list| list.join(", ")),
        }
    }
}

impl From<ModsTomlEntry> for ModFileMetadata {
    fn from(value: ModsTomlEntry) -> Self {
        Self {
            modid: value.modid,
            name: Some(value.display_name),
            version: Some(value.version),
            description: value.description,
            authors: value.authors,
        }
    }
}

pub fn parse_metadata(reader: impl Read + Seek) -> anyhow::Result<Option<ModFileMetadata>> {
    let mut zip = zip::ZipArchive::new(reader)?;

    // Used a block and let else instead of an if let to avoid what appears to be a
    // borrow checker life extension bug.
    'modstoml: {
        let Ok(mut file) = zip.by_name("META-INF/mods.toml") else { break 'modstoml };
        let mut content = String::with_capacity(file.size() as usize);
        file.read_to_string(&mut content)?;
        let modstoml = toml::from_str::<ModsToml>(&content)?;
        let mut modstoml = modstoml
            .mods
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("mcmod.info contained no mod entries"))?;
        drop(file);

        if modstoml.version == "${file.jarVersion}" {
            if let Ok(mf) = zip.by_name("META-INF/MANIFEST.MF") {
                let buffered = io::BufReader::new(mf);
                for line in buffered.lines() {
                    let line = line?;

                    if let Some((_, version)) = line.split_once("Implementation-Version: ") {
                        modstoml.version = version.to_string();
                        break;
                    }
                }
            }
        }

        return Ok(Some(modstoml.into()));
    }

    if let Ok(file) = zip.by_name("mcmod.info") {
        let mcmod: McModInfo =
            serde_json::from_reader::<_, McModInfoContainer>(file)?.try_into()?;
        return Ok(Some(mcmod.into()));
    }

    Ok(None)
}

#[cfg(test)]
mod test {
    use std::io::{Cursor, Write};

    use zip::{write::FileOptions, CompressionMethod, ZipWriter};

    use crate::domain::instance::ModFileMetadata;

    use super::parse_metadata;

    pub fn parsemeta(path: &str, content: &str) -> anyhow::Result<Option<ModFileMetadata>> {
        // write meta zip
        let mut vec = Vec::<u8>::new();
        let mut zip = ZipWriter::new(Cursor::new(&mut vec));
        let options = FileOptions::default().compression_method(CompressionMethod::Stored);
        zip.start_file(path, options)?;
        zip.write(content.as_bytes())?;
        zip.finish()?;
        drop(zip);

        // read meta zip
        let meta = parse_metadata(Cursor::new(&vec));

        Ok(meta?)
    }

    #[test]
    pub fn old_forge_metadata() -> anyhow::Result<()> {
        let mcmodinfo = r#"
          [{
            "modid": "com.test.testmod",
            "name": "TestMod",
            "description": "test desc",
            "version": "1.0.0",
            "authors": ["TestAuthor1", "TestAuthor2"]
          }]
        "#;

        let expected = Some(ModFileMetadata {
            modid: String::from("com.test.testmod"),
            name: Some(String::from("TestMod")),
            version: Some(String::from("1.0.0")),
            description: Some(String::from("test desc")),
            authors: Some(String::from("TestAuthor1, TestAuthor2")),
        });

        let returned = parsemeta("mcmod.info", mcmodinfo)?;

        assert_eq!(returned, expected);
        Ok(())
    }

    #[test]
    pub fn old_forge_metadata_partial() -> anyhow::Result<()> {
        let mcmodinfo = r#"
          [{
            "modid": "com.test.testmod"
          }]
        "#;

        let expected = Some(ModFileMetadata {
            modid: String::from("com.test.testmod"),
            name: None,
            version: None,
            description: None,
            authors: None,
        });

        let returned = parsemeta("mcmod.info", mcmodinfo)?;

        assert_eq!(returned, expected);
        Ok(())
    }

    #[test]
    pub fn new_forge_metadata() -> anyhow::Result<()> {
        let mcmodinfo = r#"
          {
            "modList": [{
              "modid": "com.test.testmod",
              "name": "TestMod",
              "description": "test desc",
              "version": "1.0.0",
              "authorList": ["TestAuthor1", "TestAuthor2"]
            }]
          }
        "#;

        let expected = Some(ModFileMetadata {
            modid: String::from("com.test.testmod"),
            name: Some(String::from("TestMod")),
            version: Some(String::from("1.0.0")),
            description: Some(String::from("test desc")),
            authors: Some(String::from("TestAuthor1, TestAuthor2")),
        });

        let returned = parsemeta("mcmod.info", mcmodinfo)?;

        assert_eq!(returned, expected);
        Ok(())
    }

    #[test]
    pub fn new_forge_metadata_partial() -> anyhow::Result<()> {
        let mcmodinfo = r#"
          {
            "modList": [{
              "modid": "com.test.testmod"
            }]
          }
        "#;

        let expected = Some(ModFileMetadata {
            modid: String::from("com.test.testmod"),
            name: None,
            version: None,
            description: None,
            authors: None,
        });

        let returned = parsemeta("mcmod.info", mcmodinfo)?;

        assert_eq!(returned, expected);
        Ok(())
    }

    #[test]
    pub fn forge_toml() -> anyhow::Result<()> {
        let modstoml = r#"[[mods]]
modId = "com.test.testmod"
version = "1.0.0"
displayName = "TestMod"
description = "test desc"
authors = "TestAuthor1, TestAuthor2"
"#;

        let expected = Some(ModFileMetadata {
            modid: String::from("com.test.testmod"),
            name: Some(String::from("TestMod")),
            version: Some(String::from("1.0.0")),
            description: Some(String::from("test desc")),
            authors: Some(String::from("TestAuthor1, TestAuthor2")),
        });

        let returned = parsemeta("META-INF/mods.toml", modstoml)?;

        assert_eq!(returned, expected);
        Ok(())
    }

    #[test]
    pub fn forge_toml_partial() -> anyhow::Result<()> {
        let modstoml = r#"[[mods]]
modId = "com.test.testmod"
version = "1.0.0"
displayName = "TestMod"
        "#;

        let expected = Some(ModFileMetadata {
            modid: String::from("com.test.testmod"),
            name: Some(String::from("TestMod")),
            version: Some(String::from("1.0.0")),
            description: None,
            authors: None,
        });

        let returned = parsemeta("META-INF/mods.toml", modstoml)?;

        assert_eq!(returned, expected);
        Ok(())
    }
}
