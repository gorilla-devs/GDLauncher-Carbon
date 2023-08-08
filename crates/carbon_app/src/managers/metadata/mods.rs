use std::{
    collections::HashMap,
    io::{self, Seek},
};

use crate::domain::instance::{info::ModLoaderType, ModFileMetadata};
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

#[derive(Deserialize, Clone)]
enum FabricEnvironmentEntry {
    #[serde(rename = "*")]
    Everywhere,
    #[serde(rename = "client")]
    Client,
    #[serde(rename = "server")]
    Server,
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
enum FabricEnvironment {
    Single(FabricEnvironmentEntry),
    List(Vec<FabricEnvironmentEntry>),
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
enum FabricEntrypoint {
    String(String),
    Object {
        #[serde(default = "fabric_quilt_adapter_default")]
        adapter: String,
        value: String,
    },
}

fn fabric_quilt_adapter_default() -> String {
    "default".to_string()
}

#[derive(Deserialize, Clone)]
struct FabricEntrypoints {
    main: Option<Vec<FabricEntrypoint>>,
    client: Option<Vec<FabricEntrypoint>>,
    server: Option<Vec<FabricEntrypoint>>,
}

#[derive(Deserialize, Clone)]
struct FabricNestedJar {
    file: String,
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
enum FabricMixin {
    String(String),
    Object {
        config: String,
        environment: Option<FabricEnvironmentEntry>,
    },
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
enum FabricVersionRange {
    Single(String),
    OrList(Vec<String>),
}

#[derive(Deserialize, Clone)]
struct FabricContact {
    email: Option<String>,
    irc: Option<String>,
    homepage: Option<String>,
    issues: Option<String>,
    sources: Option<String>,
    /// Non standard
    discord: Option<String>,
    /// Non standard
    slack: Option<String>,
    /// Non Standard
    twitter: Option<String>,
    #[serde(flatten)]
    extra: HashMap<String, String>,
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
enum FabricAuthor {
    Name(String),
    NameContact {
        name: String,
        contact: Option<String>,
    },
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
enum FabricLicense {
    Single(String),
    List(Vec<String>),
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
enum FabricIcon {
    Path(String),
    SizeMap(HashMap<String, String>),
}

#[derive(Deserialize, Clone)]
/// fabric.mod.json : Built using
/// https://fabricmc.net/wiki/documentation:fabric_mod_json_spec
/// https://fabricmc.net/wiki/documentation:fabric_mod_json_spec_old
struct FabricModJsonEntry {
    // Mandatory
    id: String,
    version: String,
    #[serde(rename = "schemaVersion")]
    #[serde(default = "fabric_mod_json_schema_version_default")]
    schema_version: u32,
    // Optional
    // Mod Loading
    provides: Option<Vec<String>>,
    environment: Option<FabricEnvironment>,
    entrypoints: Option<FabricEntrypoints>,
    jars: Option<Vec<FabricNestedJar>>,
    #[serde(rename = "languageAdapters")]
    language_adapters: Option<HashMap<String, String>>,
    mixins: Option<Vec<FabricMixin>>,
    // Dependency resolution
    depends: Option<HashMap<String, FabricVersionRange>>,
    recommends: Option<HashMap<String, FabricVersionRange>>,
    suggests: Option<HashMap<String, FabricVersionRange>>,
    breaks: Option<HashMap<String, FabricVersionRange>>,
    conflicts: Option<HashMap<String, FabricVersionRange>>,
    // Metadata
    name: Option<String>,
    description: Option<String>,
    contact: Option<FabricContact>,
    authors: Option<Vec<FabricAuthor>>,
    contributors: Option<Vec<FabricAuthor>>,
    license: Option<FabricLicense>,
    icon: Option<FabricIcon>,
}

fn fabric_mod_json_schema_version_default() -> u32 {
    0
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
enum FabricModJson {
    Single(Box<FabricModJsonEntry>),
    /// not supported by loaders >= 0.4.0
    List(Vec<FabricModJsonEntry>),
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
enum QuiltProvides {
    String(String),
    Object { id: String, version: Option<String> },
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
enum QuiltEntrypoint {
    String(String),
    Object {
        #[serde(default = " fabric_quilt_adapter_default")]
        adapter: String,
        value: String,
    },
}

#[derive(Deserialize, Clone)]
struct QuiltPlugin {
    #[serde(default = "fabric_quilt_adapter_default")]
    adapter: String,
    value: String,
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
enum QuiltVersions {
    Single(QuiltVersionsListing),
    List(Vec<QuiltVersionsListing>),
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
enum QuiltVersionsListing {
    String(String),
    Object(QuiltVersionsObject),
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
enum QuiltVersionsObject {
    Any { any: Vec<QuiltVersionsListing> },
    All { all: Vec<QuiltVersionsListing> },
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
enum QuiltDependencyListing {
    String(String),
    Object(Box<QuiltDependencyObject>),
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
enum QuiltDependency {
    Single(QuiltDependencyListing),
    List(Vec<QuiltDependencyListing>),
}

#[derive(Deserialize, Clone)]
struct QuiltDependencyObject {
    id: String,
    #[serde(default = "quilt_dependency_versions_default")]
    versions: QuiltVersions,
    reason: Option<String>,
    #[serde(default = "quilt_dependency_optional_default")]
    optional: bool,
    unless: Option<QuiltDependency>,
}

fn quilt_dependency_versions_default() -> QuiltVersions {
    QuiltVersions::Single(QuiltVersionsListing::String(String::from("*")))
}

fn quilt_dependency_optional_default() -> bool {
    false
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
enum QuiltLoadType {
    Always,
    IfPossible,
    IfRequired,
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
enum QuiltContributorRole {
    Single(String),
    List(Vec<String>),
}

#[derive(Deserialize, Clone)]
struct QuiltContactInfo {
    email: Option<String>,
    homepage: Option<String>,
    issues: Option<String>,
    sources: Option<String>,
    /// Non standard
    discord: Option<String>,
    /// Non standard
    slack: Option<String>,
    /// Non Standard
    twitter: Option<String>,
    #[serde(flatten)]
    extra: HashMap<String, String>,
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
enum QuiltLicense {
    Identifier(String),
    License {
        name: String,
        id: String,
        url: String,
        description: Option<String>,
    },
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
enum QuiltIcon {
    Path(String),
    SizeMap(HashMap<String, String>),
}

#[derive(Deserialize, Clone)]
struct QuiltLoaderMetadata {
    name: Option<String>,
    description: Option<String>,
    contributors: Option<indexmap::IndexMap<String, QuiltContributorRole>>,
    contact: Option<QuiltContactInfo>,
    license: Option<QuiltLicense>,
    icon: Option<QuiltIcon>,
}
#[derive(Deserialize, Clone)]
struct QuiltLoader {
    group: String,
    id: String,
    provides: Option<QuiltProvides>,
    version: String,
    entrypoints: Option<HashMap<String, QuiltEntrypoint>>,
    plugins: Option<Vec<QuiltPlugin>>,
    jars: Option<Vec<String>>,
    language_adapters: Option<HashMap<String, String>>,
    depends: Option<Vec<QuiltDependencyListing>>,
    breaks: Option<Vec<QuiltDependencyListing>>,
    load_type: Option<QuiltLoadType>,
    repositories: Option<Vec<String>>,
    #[serde(default = "quilt_intermediate_mappings_default")]
    intermediate_mappings: String,
    metadata: Option<QuiltLoaderMetadata>,
}

fn quilt_intermediate_mappings_default() -> String {
    String::from("org.quiltmc:hashed")
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
enum QuiltPathListing {
    Single(String),
    List(Vec<String>),
}

#[derive(Deserialize, Clone)]
enum QuiltMinecraftEnvironment {
    #[serde(rename = "*")]
    All,
    #[serde(rename = "client")]
    Client,
    #[serde(rename = "dedicated_server")]
    DedicatedServer,
}

#[derive(Deserialize, Clone)]
struct QuiltMinecraftSection {
    environment: Option<QuiltMinecraftEnvironment>,
}

#[derive(Deserialize, Clone)]
/// quilt.mod.json : built using
/// https://github.com/QuiltMC/rfcs/blob/master/specification/0002-quilt.mod.json.md
/// https://raw.githubusercontent.com/QuiltMC/quilt-json-schemas/main/quilt.mod.json/schemas/main.json
/// https://raw.githubusercontent.com/QuiltMC/quilt-json-schemas/main/quilt.mod.json/schemas/schema_version_1.json
struct QuiltModJson {
    schema_version: u32,
    quilt_loader: QuiltLoader,
    mixin: Option<QuiltPathListing>,
    access_widener: Option<QuiltPathListing>,
    minecraft: Option<QuiltMinecraftSection>,
}

impl From<McModInfo> for ModFileMetadata {
    fn from(value: McModInfo) -> Self {
        Self {
            modid: value.modid,
            name: value.name,
            version: value.version,
            description: value.description,
            authors: value.authors.map(|list| list.join(", ")),
            modloaders: vec![ModLoaderType::Forge],
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
            modloaders: vec![ModLoaderType::Forge],
        }
    }
}

impl TryFrom<FabricModJson> for ModFileMetadata {
    type Error = anyhow::Error;

    fn try_from(value: FabricModJson) -> Result<Self, Self::Error> {
        fn flatten_authors(authors: Vec<FabricAuthor>) -> Option<String> {
            let authors_string = authors
                .into_iter()
                .map(|author| match author {
                    FabricAuthor::Name(name) => name,
                    FabricAuthor::NameContact { name, contact } => {
                        if let Some(contact) = contact {
                            format!("{} <{}>", name, contact)
                        } else {
                            name
                        }
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            Some(authors_string)
        }

        match value {
            FabricModJson::Single(info) => Ok(Self {
                modid: info.id,
                name: info.name,
                version: Some(info.version),
                description: info.description,
                authors: info.authors.and_then(flatten_authors),
                modloaders: vec![ModLoaderType::Fabric],
            }),
            FabricModJson::List(list) => {
                let info = list
                    .get(0)
                    .ok_or_else(|| anyhow!("fabric.mod.json entry list should not be empty"))?;
                Ok(Self {
                    modid: info.id.clone(),
                    name: info.name.clone(),
                    version: Some(info.version.clone()),
                    description: info.description.clone(),
                    authors: info.authors.clone().and_then(flatten_authors),
                    modloaders: vec![ModLoaderType::Fabric],
                })
            }
        }
    }
}

impl From<QuiltModJson> for ModFileMetadata {
    fn from(value: QuiltModJson) -> Self {
        let (name, description, authors) = if let Some(metadata) = value.quilt_loader.metadata {
            let authors = metadata.contributors.map(|contributors| {
                let authors_string = contributors.keys().cloned().collect::<Vec<_>>().join(", ");
                authors_string
            });
            (metadata.name.clone(), metadata.description, authors)
        } else {
            (None, None, None)
        };
        Self {
            modid: value.quilt_loader.id,
            name,
            version: Some(value.quilt_loader.version),
            description,
            authors,
            modloaders: vec![ModLoaderType::Quilt],
        }
    }
}

fn merge_mod_metadata(
    metadata: Option<ModFileMetadata>,
    mut other: ModFileMetadata,
) -> Option<ModFileMetadata> {
    match metadata {
        Some(metadata) => Some(ModFileMetadata {
            modid: metadata.modid,
            name: metadata.name.or(other.name),
            version: metadata.version.or(other.version),
            description: metadata.description.or(other.description),
            authors: metadata.authors.or(other.authors),
            modloaders: {
                let mut modloaders = metadata.modloaders;
                modloaders.append(&mut other.modloaders);
                modloaders
            },
        }),
        None => Some(other),
    }
}

pub fn parse_metadata(reader: impl Read + Seek) -> anyhow::Result<Option<ModFileMetadata>> {
    let mut zip = zip::ZipArchive::new(reader)?;

    let mut mod_metadata: Option<ModFileMetadata> = None;

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

        let mut metadata: ModFileMetadata = modstoml.into();
        match metadata.version {
            Some(version) if version == "${file.jarVersion}" => {
                metadata.version = None;
            }
            _ => (),
        }

        mod_metadata = merge_mod_metadata(mod_metadata, metadata);
    }

    'fabric_mod_json: {
        let Ok(mut file) = zip.by_name("fabric.mod.json") else { break 'fabric_mod_json };
        let mut content = String::with_capacity(file.size() as usize);
        file.read_to_string(&mut content)?;

        let fabric_mod_json = serde_json::from_str::<FabricModJson>(&content)?;

        mod_metadata = merge_mod_metadata(mod_metadata, fabric_mod_json.try_into()?);
    }

    'quilt_mod_json: {
        let Ok(mut file) = zip.by_name("quilt.mod.json") else { break 'quilt_mod_json };
        let mut content = String::with_capacity(file.size() as usize);
        file.read_to_string(&mut content)?;

        let quilt_mod_json = serde_json::from_str::<QuiltModJson>(&content)?;

        mod_metadata = merge_mod_metadata(mod_metadata, quilt_mod_json.into());
    }

    if let Ok(file) = zip.by_name("mcmod.info") {
        let mcmod: McModInfo =
            serde_json::from_reader::<_, McModInfoContainer>(file)?.try_into()?;
        mod_metadata = merge_mod_metadata(mod_metadata, mcmod.into());
    }

    Ok(mod_metadata)
}

#[cfg(test)]
mod test {
    use std::io::{Cursor, Write};

    use zip::{write::FileOptions, CompressionMethod, ZipWriter};

    use crate::domain::instance::{info::ModLoaderType, ModFileMetadata};

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

        meta
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
            modloaders: vec![ModLoaderType::Forge],
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
            modloaders: vec![ModLoaderType::Forge],
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
            modloaders: vec![ModLoaderType::Forge],
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
            modloaders: vec![ModLoaderType::Forge],
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
            modloaders: vec![ModLoaderType::Forge],
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
            modloaders: vec![ModLoaderType::Forge],
        });

        let returned = parsemeta("META-INF/mods.toml", modstoml)?;

        assert_eq!(returned, expected);
        Ok(())
    }

    #[test]
    pub fn fabric_mod_json() -> anyhow::Result<()> {
        let modjson = r#"{
  "schemaVersion": 1,
  "id": "com.test.testmod",
  "version": "1.0.0",

  "name": "TestMod",
  "description": "This is an example description!",
  "authors": [
    "TestAuthor1",
    "TestAuthor2"
  ],
  "contact": {
    "homepage": "https://gdlauncher.com/",
    "sources": "https://github.com/grilla-devs/fabric-test-mod"
  },

  "license": "CC0-1.0",
  "icon": "assets/modid/icon.png",

  "environment": "*",
  "entrypoints": {
    "main": [
      "uk.co.xsc.TestMod"
    ],
    "client": [
      "uk.co.xsc.ClientModInitializer"
    ]
  },
  "mixins": [
    "modid.mixins.json"
  ],

  "requires": {
    "fabricloader": ">=0.4.0",
    "fabric": "*"
  },
  "suggests": {
    "foobob": "*"
  }
}
        "#;

        let expected = Some(ModFileMetadata {
            modid: String::from("com.test.testmod"),
            name: Some(String::from("TestMod")),
            version: Some(String::from("1.0.0")),
            description: Some(String::from("This is an example description!")),
            authors: Some(String::from("TestAuthor1, TestAuthor2")),
            modloaders: vec![ModLoaderType::Fabric],
        });

        let returned = parsemeta("fabric.mod.json", modjson)?;

        assert_eq!(returned, expected);

        Ok(())
    }

    #[test]
    pub fn quilt_mod_json() -> anyhow::Result<()> {
        let modjson = r#"{
	"schema_version": 1,
	"quilt_loader": {
		"group": "com.test.testmod",
		"id": "com.test.testmod",
		"version": "1.0.0",
		"metadata": {
			"name": "TestMod",
			"description": "A short description of your mod.",
			"contributors": {
				"TestAuthor1": "Owner",
				"TestAuthor2": "SomeOtherRole"
			},
			"contact": {
				"homepage": "https://gdlauncher.com/",
				"issues": "https://github.com/grilla-devs/quilt-test-mod/issues",
				"sources": "https://github.com/grilla-devs/quilt-test-mod"
			},
			"icon": "assets/test_mod/icon.png"
		},
		"intermediate_mappings": "net.fabricmc:intermediary",
		"entrypoints": {
			"init": "com.test.testmod.TestMod"
		},
		"depends": [
			{
				"id": "quilt_loader",
				"versions": ">=0.19.1"
			},
			{
				"id": "quilted_fabric_api",
				"versions": ">=7.0.2"
			},
			{
				"id": "minecraft",
				"versions": ">=1.20"
			}
		]
	},
	"mixin": "test_mod.mixins.json"
}
        "#;

        let expected = Some(ModFileMetadata {
            modid: String::from("com.test.testmod"),
            name: Some(String::from("TestMod")),
            version: Some(String::from("1.0.0")),
            description: Some(String::from("A short description of your mod.")),
            authors: Some(String::from("TestAuthor1, TestAuthor2")),
            modloaders: vec![ModLoaderType::Quilt],
        });

        let returned = parsemeta("quilt.mod.json", modjson)?;

        assert_eq!(returned, expected);

        Ok(())
    }
}
