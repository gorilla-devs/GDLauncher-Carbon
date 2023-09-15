use std::{
    collections::HashMap,
    io::{self, Seek},
    num::ParseIntError,
};

use crate::domain::instance::{self as domain, info::ModLoaderType};
use anyhow::{anyhow, bail};
use serde::Deserialize;
use std::io::{BufRead, Read};

#[derive(Deserialize)]
#[serde(untagged)]
enum McModInfoContainer {
    Old(Vec<McModInfo>),
    New(NewMcModInfo),
}

#[derive(Deserialize)]
// mcmod.info (new)
// https://github.com/MinecraftForge/FML/wiki/FML-mod-information-file/c8d8f1929aff9979e322af79a59ce81f3e02db6a
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
    #[serde(rename = "logoFile")]
    logo_file: Option<String>,
}

#[derive(Deserialize)]
// mcmod.info (old)
// https://github.com/MinecraftForge/FML/wiki/FML-mod-information-file/5bf6a2d05145ec79387acc0d45c958642fb049fc
struct McModInfo {
    modid: String,
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
    authors: Option<Vec<String>>,
    #[serde(rename = "logoFile")]
    logo_file: Option<String>,
}

impl From<NewMcModInfoObj> for McModInfo {
    fn from(value: NewMcModInfoObj) -> Self {
        Self {
            modid: value.modid,
            name: value.name,
            version: value.version,
            description: value.description,
            authors: value.authors,
            logo_file: value.logo_file,
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
// mods.toml
// https://github.com/MinecraftForge/Documentation/blob/5ab4ba6cf9abc0ac4c0abd96ad187461aefd72af/docs/gettingstarted/structuring.md
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
    #[serde(rename = "logoFile")]
    logo_file: Option<String>,
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

impl FabricIcon {
    fn pick_best(self) -> Option<String> {
        match self {
            Self::Path(path) => Some(path),
            Self::SizeMap(map) => pick_best_icon(map),
        }
    }
}

fn pick_best_icon(icons: HashMap<String, String>) -> Option<String> {
    let mut icons = icons
        .into_iter()
        .map(|(width, path)| Ok((width.parse::<u32>()?, path)))
        .collect::<Result<Vec<_>, ParseIntError>>()
        .ok()?;

    icons.sort_by_key(|(width, _)| *width);
    let mut best = Option::<(u32, String)>::None;

    for (width, path) in icons.into_iter().rev() {
        match best {
            None => best = Some((width, path)),
            Some((old_width, _)) if width < old_width && width >= 45 => best = Some((width, path)),
            _ => {}
        }
    }

    best.map(|(_, path)| path)
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

impl QuiltIcon {
    fn pick_best(self) -> Option<String> {
        match self {
            Self::Path(path) => Some(path),
            Self::SizeMap(map) => pick_best_icon(map),
        }
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModFileMetadata {
    pub modid: String,
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub authors: Option<String>,
    pub modloaders: Vec<domain::info::ModLoaderType>,
    pub logo_file: Option<String>,
}

impl From<ModFileMetadata> for domain::ModFileMetadata {
    fn from(value: ModFileMetadata) -> Self {
        Self {
            modid: value.modid,
            name: value.name,
            version: value.version,
            description: value.description,
            authors: value.authors,
            modloaders: value.modloaders,
        }
    }
}

impl From<McModInfo> for ModFileMetadata {
    fn from(value: McModInfo) -> Self {
        Self {
            modid: value.modid,
            name: value.name,
            version: value.version,
            description: value.description,
            authors: value.authors.map(|list| list.join(", ")),
            logo_file: value.logo_file,
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
            logo_file: value.logo_file,
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
                logo_file: info.icon.map(FabricIcon::pick_best).flatten(),
                modloaders: vec![ModLoaderType::Fabric],
            }),
            FabricModJson::List(mut list) => {
                if list.is_empty() {
                    bail!("fabric.mod.json entry list should not be empty");
                }

                let info = list.swap_remove(0);
                Ok(Self {
                    modid: info.id.clone(),
                    name: info.name.clone(),
                    version: Some(info.version.clone()),
                    description: info.description.clone(),
                    authors: info.authors.clone().and_then(flatten_authors),
                    logo_file: info.icon.map(FabricIcon::pick_best).flatten(),
                    modloaders: vec![ModLoaderType::Fabric],
                })
            }
        }
    }
}

impl From<QuiltModJson> for ModFileMetadata {
    fn from(value: QuiltModJson) -> Self {
        let (name, description, authors, icon) = if let Some(metadata) = value.quilt_loader.metadata
        {
            let authors = metadata.contributors.map(|contributors| {
                let authors_string = contributors.keys().cloned().collect::<Vec<_>>().join(", ");
                authors_string
            });
            (
                metadata.name.clone(),
                metadata.description,
                authors,
                metadata.icon,
            )
        } else {
            (None, None, None, None)
        };
        Self {
            modid: value.quilt_loader.id,
            name,
            version: Some(value.quilt_loader.version),
            description,
            authors,
            logo_file: icon.map(QuiltIcon::pick_best).flatten(),
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
            logo_file: metadata.logo_file.or(other.logo_file),
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
        let Ok(mut file) = zip.by_name("META-INF/mods.toml") else {
            break 'modstoml;
        };
        let mut content = String::with_capacity(file.size() as usize);
        file.read_to_string(&mut content)?;
        let modstoml = toml::from_str::<ModsToml>(&content)?;
        let mut modstoml = modstoml
            .mods
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("mods.toml contained no mod entries"))?;
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
        let Ok(mut file) = zip.by_name("fabric.mod.json") else {
            break 'fabric_mod_json;
        };
        let mut content = String::with_capacity(file.size() as usize);
        file.read_to_string(&mut content)?;

        let fabric_mod_json = serde_json::from_str::<FabricModJson>(&content)?;

        mod_metadata = merge_mod_metadata(mod_metadata, fabric_mod_json.try_into()?);
    }

    'quilt_mod_json: {
        let Ok(mut file) = zip.by_name("quilt.mod.json") else {
            break 'quilt_mod_json;
        };
        let mut content = String::with_capacity(file.size() as usize);
        file.read_to_string(&mut content)?;

        let quilt_mod_json = serde_json::from_str::<QuiltModJson>(&content)?;

        mod_metadata = merge_mod_metadata(mod_metadata, quilt_mod_json.into());
    }

    'mcmodinfo: {
        let Ok(file) = zip.by_name("mcmod.info") else {
            break 'mcmodinfo;
        };

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

    use crate::domain::instance::info::ModLoaderType;

    use super::{parse_metadata, ModFileMetadata};

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
            "authors": ["TestAuthor1", "TestAuthor2"],
            "logoFile": "/test/logo"
          }]
        "#;

        let expected = Some(ModFileMetadata {
            modid: String::from("com.test.testmod"),
            name: Some(String::from("TestMod")),
            version: Some(String::from("1.0.0")),
            description: Some(String::from("test desc")),
            authors: Some(String::from("TestAuthor1, TestAuthor2")),
            logo_file: Some(String::from("/test/logo")),
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
            logo_file: None,
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
              "authorList": ["TestAuthor1", "TestAuthor2"],
              "logoFile": "/test/logo"
            }]
          }
        "#;

        let expected = Some(ModFileMetadata {
            modid: String::from("com.test.testmod"),
            name: Some(String::from("TestMod")),
            version: Some(String::from("1.0.0")),
            description: Some(String::from("test desc")),
            authors: Some(String::from("TestAuthor1, TestAuthor2")),
            logo_file: Some(String::from("/test/logo")),
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
            logo_file: None,
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
logoFile = "test/logo"
        "#;

        let expected = Some(ModFileMetadata {
            modid: String::from("com.test.testmod"),
            name: Some(String::from("TestMod")),
            version: Some(String::from("1.0.0")),
            description: Some(String::from("test desc")),
            authors: Some(String::from("TestAuthor1, TestAuthor2")),
            logo_file: Some(String::from("test/logo")),
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
            logo_file: None,
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
  "icon": {
    "1000": "assets/modid/icon1000.png",
    "32": "assets/modid/icon32.png",
    "75": "assets/modid/icon75.png",
    "100": "assets/modid/icon100.png"
  },

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
            logo_file: Some(String::from("assets/modid/icon75.png")),
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
            logo_file: Some(String::from("assets/test_mod/icon.png")),
            modloaders: vec![ModLoaderType::Quilt],
        });

        let returned = parsemeta("quilt.mod.json", modjson)?;

        assert_eq!(returned, expected);

        Ok(())
    }
}
