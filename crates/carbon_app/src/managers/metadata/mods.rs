use std::io::{BufRead, Read};
use std::{
    collections::HashMap,
    io::{self, Seek},
    num::ParseIntError,
};

fn sanitize_json_content(content: &str) -> String {
    content
        .chars()
        .filter(|c| !c.is_control() || *c == '\t' || *c == '\n' || *c == '\r')
        .collect()
}

/// The most that is read of a mod's descriptor (`mods.toml`,
/// `fabric.mod.json` and the like) or of its manifest. Real ones are a few
/// KB; the limit only has to stop an entry made to fill memory.
pub(super) const DESCRIPTOR_LIMIT: u64 = 16 * 1024 * 1024;
/// The most that is read of a mod's icon.
pub(super) const ICON_LIMIT: u64 = 64 * 1024 * 1024;

/// Reads an archive entry whole, or gives `None` when it cannot be read or
/// holds more than `limit` bytes.
///
/// The size an archive declares for an entry is whatever its author wrote
/// there, and a few KB of deflate can inflate to gigabytes. So nothing is
/// reserved from the declared size (an allocation the machine cannot give
/// aborts the process instead of returning an error) and nothing past the
/// limit is read.
fn read_entry(entry: impl Read, limit: u64) -> Option<Vec<u8>> {
    let mut content = Vec::new();
    entry
        .take(limit.saturating_add(1))
        .read_to_end(&mut content)
        .ok()?;
    (content.len() as u64 <= limit).then_some(content)
}

/// [`read_entry`] for a descriptor, which is text.
fn read_descriptor(entry: impl Read) -> Option<String> {
    String::from_utf8(read_entry(entry, DESCRIPTOR_LIMIT)?).ok()
}

/// The icon a mod's descriptor names, read from the mod's archive. A missing,
/// damaged or oversized icon only costs the icon, not the addon.
pub(super) fn read_logo(reader: &mut (impl Read + Seek), logo_file: &str) -> Option<Vec<u8>> {
    let mut zip = zip::ZipArchive::new(reader).ok()?;
    let entry = zip.by_name(logo_file).ok()?;
    let image = read_entry(entry, ICON_LIMIT);
    if image.is_none() {
        tracing::debug!("could not read the icon {logo_file}, or it is too large");
    }
    image
}

use anyhow::{anyhow, bail};
use serde::{Deserialize, Deserializer};
use zip::read::ZipFile;

use crate::domain::instance::{self as domain, info::ModLoaderType};

fn deserialize_authors_flexible<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum AuthorsField {
        String(String),
        Array(Vec<String>),
    }

    let authors = Option::<AuthorsField>::deserialize(deserializer)?;
    Ok(match authors {
        Some(AuthorsField::String(s)) => Some(s),
        Some(AuthorsField::Array(vec)) => Some(vec.join(", ")),
        None => None,
    })
}

#[derive(Deserialize)]
#[serde(untagged)]
enum McModInfoContainer {
    Old(Vec<McModInfo>),
    New(NewMcModInfo),
}

impl Default for McModInfoContainer {
    fn default() -> Self {
        Self::New(NewMcModInfo::default())
    }
}

#[derive(Deserialize, Default)]
// mcmod.info (new)
// https://github.com/MinecraftForge/FML/wiki/FML-mod-information-file/c8d8f1929aff9979e322af79a59ce81f3e02db6a
struct NewMcModInfo {
    #[serde(rename = "modList")]
    mod_list: Vec<NewMcModInfoObj>,
}

#[derive(Deserialize, Default)]
struct NewMcModInfoObj {
    modid: Option<String>,
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
    #[serde(rename = "authorList")]
    authors: Option<Vec<String>>,
    #[serde(rename = "logoFile")]
    logo_file: Option<String>,
}

#[derive(Deserialize, Default, Debug)]
// mcmod.info (old)
// https://github.com/MinecraftForge/FML/wiki/FML-mod-information-file/5bf6a2d05145ec79387acc0d45c958642fb049fc
struct McModInfo {
    modid: Option<String>,
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
    #[serde(alias = "authorList")] // cccmod.info uses authorList instead of authors
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
    modid: Option<String>,
    version: String,
    #[serde(rename = "displayName")]
    display_name: String,
    description: Option<String>,
    #[serde(deserialize_with = "deserialize_authors_flexible", default)]
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
    NameContactObject {
        name: String,
        contact: Option<FabricContact>,
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
    #[serde(rename = "accessWidener")]
    access_widener: Option<String>,
    custom: Option<serde_json::Value>,
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
        #[serde(default = "fabric_quilt_adapter_default")]
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
    pub modid: Option<String>,
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub authors: Option<String>,
    pub modloaders: Vec<domain::info::ModLoaderType>,
    pub logo_file: Option<String>,
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
                    FabricAuthor::NameContactObject { name, contact } => {
                        if let Some(contact_obj) = contact {
                            if let Some(email) = &contact_obj.email {
                                format!("{} <{}>", name, email)
                            } else if let Some(homepage) = &contact_obj.homepage {
                                format!("{} <{}>", name, homepage)
                            } else {
                                name
                            }
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
                modid: Some(info.id),
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
                    modid: Some(info.id.clone()),
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
            modid: Some(value.quilt_loader.id),
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
            modid: metadata.modid.or(other.modid),
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

pub fn parse_metadata(reader: &mut (impl Read + Seek)) -> anyhow::Result<Option<ModFileMetadata>> {
    let mut zip = zip::ZipArchive::new(reader)?;

    let mut mod_metadata: Option<ModFileMetadata> = None;

    // Used a block and let else instead of an if let to avoid what appears to be a
    // borrow checker life extension bug.
    'modstoml: {
        let Ok(file) = zip.by_name("META-INF/mods.toml") else {
            break 'modstoml;
        };
        let Some(content) = read_descriptor(file) else {
            break 'modstoml;
        };

        let sanitized_content = sanitize_json_content(&content);
        let Ok(modstoml) = toml::from_str::<ModsToml>(&sanitized_content) else {
            // mods.toml exists but failed to parse - try neoforge.mods.toml instead
            break 'modstoml;
        };
        let Some(mut modstoml) = modstoml.mods.into_iter().next() else {
            // mods.toml has no entries - try neoforge.mods.toml instead
            break 'modstoml;
        };

        if modstoml.version == "${file.jarVersion}" {
            if let Ok(mf) = zip.by_name("META-INF/MANIFEST.MF") {
                let buffered = io::BufReader::new(mf.take(DESCRIPTOR_LIMIT));
                for line in buffered.lines().map_while(Result::ok) {
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

    'neoforge_mods_toml: {
        let Ok(file) = zip.by_name("META-INF/neoforge.mods.toml") else {
            break 'neoforge_mods_toml;
        };
        let Some(content) = read_descriptor(file) else {
            break 'neoforge_mods_toml;
        };

        let sanitized_content = sanitize_json_content(&content);
        let Ok(modstoml) = toml::from_str::<ModsToml>(&sanitized_content) else {
            break 'neoforge_mods_toml;
        };
        let Some(mut modstoml) = modstoml.mods.into_iter().next() else {
            break 'neoforge_mods_toml;
        };

        let mut metadata: ModFileMetadata = modstoml.into();
        metadata.modloaders = vec![ModLoaderType::Neoforge];
        match metadata.version {
            Some(version) if version == "${file.jarVersion}" => {
                metadata.version = None;
            }
            _ => (),
        }

        mod_metadata = merge_mod_metadata(mod_metadata, metadata);
    }

    'fabric_mod_json: {
        let Ok(file) = zip.by_name("fabric.mod.json") else {
            break 'fabric_mod_json;
        };
        let Some(content) = read_descriptor(file) else {
            break 'fabric_mod_json;
        };

        let sanitized_content = sanitize_json_content(&content);
        let Ok(fabric_mod_json) = serde_json::from_str::<FabricModJson>(&sanitized_content) else {
            break 'fabric_mod_json;
        };
        let Ok(metadata) = fabric_mod_json.try_into() else {
            break 'fabric_mod_json;
        };

        mod_metadata = merge_mod_metadata(mod_metadata, metadata);
    }

    'quilt_mod_json: {
        let Ok(file) = zip.by_name("quilt.mod.json") else {
            break 'quilt_mod_json;
        };
        let Some(content) = read_descriptor(file) else {
            break 'quilt_mod_json;
        };

        let sanitized_content = sanitize_json_content(&content);
        let Ok(quilt_mod_json) = serde_json::from_str::<QuiltModJson>(&sanitized_content) else {
            break 'quilt_mod_json;
        };

        mod_metadata = merge_mod_metadata(mod_metadata, quilt_mod_json.into());
    }

    'mcmodinfo: {
        let mut file: Option<ZipFile<_>> = None;

        // CodeChickenCore for some unearthly reason uses cccmod.info instead of mcmod.info
        // https://github.com/Chicken-Bones/CodeChickenCore/blob/master/resources/cccmod.info
        let valid_names = ["mcmod.info", "cccmod.info"];

        let file_index = valid_names
            .iter()
            .position(|&name| zip.by_name(name).is_ok());

        if let Some(index) = file_index {
            file = zip.by_name(valid_names[index]).ok();
        }

        if file.is_none() {
            break 'mcmodinfo;
        }

        let file = file.expect("we just checked this");
        // One that cannot be read counts as an empty one, as it always has.
        let content = read_descriptor(file).unwrap_or_default();

        let sanitized_content = sanitize_json_content(&content);
        let mcmod: McModInfo = serde_json::from_str::<McModInfoContainer>(&sanitized_content)
            .unwrap_or_default()
            .try_into()
            .unwrap_or_default();

        mod_metadata = merge_mod_metadata(mod_metadata, mcmod.into());
    }

    Ok(mod_metadata)
}

#[cfg(test)]
pub(in crate::managers::metadata) mod test {
    use std::io::{Cursor, Write};

    use zip::{CompressionMethod, ZipWriter, write::FileOptions};

    use crate::domain::instance::info::ModLoaderType;

    use super::{ModFileMetadata, parse_metadata};

    pub fn parsemeta(path: &str, content: &str) -> anyhow::Result<Option<ModFileMetadata>> {
        // write meta zip
        let mut vec = Vec::<u8>::new();
        let mut zip = ZipWriter::new(Cursor::new(&mut vec));
        let options = FileOptions::<()>::default().compression_method(CompressionMethod::Stored);
        zip.start_file(path, options)?;
        zip.write_all(content.as_bytes())?;
        zip.finish()?;

        // read meta zip
        let meta = parse_metadata(&mut Cursor::new(&vec));

        meta
    }

    /// A jar with these entries, deflated.
    pub fn jar(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut vec = Vec::<u8>::new();
        let mut zip = ZipWriter::new(Cursor::new(&mut vec));
        let options = FileOptions::<()>::default().compression_method(CompressionMethod::Deflated);
        for (path, content) in entries {
            zip.start_file(*path, options).unwrap();
            zip.write_all(content).unwrap();
        }
        zip.finish().unwrap();
        vec
    }

    /// Rewrites the central directory of a one-entry archive so that the
    /// entry declares `size` bytes of content through a zip64 extra field,
    /// as a crafted archive does. The entry's real content is untouched.
    pub fn declare_size(archive: &[u8], size: u64) -> Vec<u8> {
        let u16_at = |at: usize| u16::from_le_bytes(archive[at..at + 2].try_into().unwrap());
        let u32_at = |at: usize| u32::from_le_bytes(archive[at..at + 4].try_into().unwrap());
        let end = archive
            .windows(4)
            .rposition(|bytes| bytes == b"PK\x05\x06")
            .expect("end of central directory");
        assert_eq!(u16_at(end + 10), 1, "one entry");
        let directory = u32_at(end + 16) as usize;
        assert_eq!(&archive[directory..directory + 4], b"PK\x01\x02");
        let name = u16_at(directory + 28) as usize;
        assert_eq!(u16_at(directory + 30), 0, "the fixture has no extra field");
        assert_eq!(u16_at(directory + 32), 0, "the fixture has no comment");
        let mut entry = archive[directory..directory + 46 + name].to_vec();
        // Uncompressed size: "see the zip64 extra field".
        entry[24..28].copy_from_slice(&u32::MAX.to_le_bytes());
        entry[30..32].copy_from_slice(&12u16.to_le_bytes());
        entry.extend_from_slice(&1u16.to_le_bytes());
        entry.extend_from_slice(&8u16.to_le_bytes());
        entry.extend_from_slice(&size.to_le_bytes());
        let mut forged = archive[..directory].to_vec();
        forged.extend_from_slice(&entry);
        let mut tail = archive[end..].to_vec();
        tail[12..16].copy_from_slice(&(entry.len() as u32).to_le_bytes());
        forged.extend_from_slice(&tail);
        forged
    }

    const FABRIC: &[u8] = br#"{"id":"example","name":"Example","version":"1.0.0"}"#;

    /// An archive can declare any size for an entry. Reserving memory for a
    /// declared size the machine cannot give ends the whole core, on every
    /// start, until the file is deleted by hand.
    #[test]
    fn a_descriptor_that_declares_an_impossible_size_is_read_for_what_it_holds() {
        let forged = declare_size(&jar(&[("fabric.mod.json", FABRIC)]), 1 << 60);
        let mut archive = zip::ZipArchive::new(Cursor::new(&forged)).unwrap();
        assert_eq!(
            archive.by_name("fabric.mod.json").unwrap().size(),
            1 << 60,
            "the fixture declares the forged size"
        );

        let parsed = parse_metadata(&mut Cursor::new(&forged)).unwrap();
        // The entry's content does not match what it declares, so whether
        // the reader hands it over is its own business; the scan goes on
        // either way.
        if let Some(parsed) = parsed {
            assert_eq!(parsed.modid.as_deref(), Some("example"));
        }
    }

    /// A few KB that inflate to more than the limit are not read into
    /// memory; the other descriptors of the jar still are.
    #[test]
    fn a_descriptor_that_inflates_past_the_limit_is_treated_as_missing() {
        let huge = vec![b' '; super::DESCRIPTOR_LIMIT as usize + 1];
        for path in [
            "META-INF/mods.toml",
            "META-INF/neoforge.mods.toml",
            "fabric.mod.json",
            "quilt.mod.json",
            "mcmod.info",
        ] {
            let bomb = jar(&[(path, &huge)]);
            assert!(bomb.len() < 1024 * 1024, "{path}: the jar itself is small");
            assert_eq!(
                parse_metadata(&mut Cursor::new(&bomb)).unwrap(),
                if path == "mcmod.info" {
                    // An unreadable mcmod.info has always counted as an
                    // empty one.
                    parsemeta("mcmod.info", "").unwrap()
                } else {
                    None
                },
                "{path}"
            );
        }
        let mixed = jar(&[("quilt.mod.json", &huge), ("fabric.mod.json", FABRIC)]);
        let parsed = parse_metadata(&mut Cursor::new(&mixed)).unwrap().unwrap();
        assert_eq!(parsed.modid.as_deref(), Some("example"));
    }

    #[test]
    fn a_descriptor_exactly_at_the_limit_is_still_read() {
        let mut content = FABRIC.to_vec();
        content.resize(super::DESCRIPTOR_LIMIT as usize, b' ');
        let parsed = parse_metadata(&mut Cursor::new(&jar(&[("fabric.mod.json", &content)])))
            .unwrap()
            .unwrap();
        assert_eq!(parsed.modid.as_deref(), Some("example"));
    }

    /// The version of a Forge mod can come from the jar's manifest, which is
    /// read line by line: one endless line must not fill memory either.
    #[test]
    fn a_manifest_that_is_one_endless_line_does_not_give_a_version() {
        let toml = b"[[mods]]\nmodId = 'example'\nversion = '${file.jarVersion}'\ndisplayName = 'Example'\n";
        let mut manifest = vec![b'x'; super::DESCRIPTOR_LIMIT as usize + 1];
        manifest.extend_from_slice(b"\nImplementation-Version: 9.9.9\n");
        let parsed = parse_metadata(&mut Cursor::new(&jar(&[
            ("META-INF/mods.toml", toml),
            ("META-INF/MANIFEST.MF", &manifest),
        ])))
        .unwrap()
        .unwrap();
        assert_eq!(parsed.modid.as_deref(), Some("example"));
        assert_eq!(parsed.version, None);

        let ordinary = b"Manifest-Version: 1.0\nImplementation-Version: 2.3.4\n";
        let parsed = parse_metadata(&mut Cursor::new(&jar(&[
            ("META-INF/mods.toml", toml),
            ("META-INF/MANIFEST.MF", ordinary),
        ])))
        .unwrap()
        .unwrap();
        assert_eq!(parsed.version.as_deref(), Some("2.3.4"));
    }

    #[test]
    fn an_icon_is_read_up_to_its_limit_whatever_size_it_declares() {
        let icon = b"\x89PNG fixture";
        let ordinary = jar(&[("assets/logo.png", icon)]);
        assert_eq!(
            super::read_logo(&mut Cursor::new(&ordinary), "assets/logo.png").as_deref(),
            Some(&icon[..])
        );
        assert_eq!(
            super::read_logo(&mut Cursor::new(&ordinary), "missing.png"),
            None
        );

        // Declares 2^60 bytes: no memory is reserved for that.
        let forged = declare_size(&ordinary, 1 << 60);
        if let Some(read) = super::read_logo(&mut Cursor::new(&forged), "assets/logo.png") {
            assert_eq!(read, icon);
        }

        // Inflates past the limit: no icon, and the addon is still listed.
        let huge = vec![0u8; super::ICON_LIMIT as usize + 1];
        let bomb = jar(&[("assets/logo.png", &huge)]);
        assert!(bomb.len() < 1024 * 1024);
        assert_eq!(
            super::read_logo(&mut Cursor::new(&bomb), "assets/logo.png"),
            None
        );
    }

    #[test]
    fn neoforge_source_identifies_neoforge() {
        let body = "[[mods]]\nmodId = 'example'\nversion = '1.0.0'\ndisplayName = 'Example'\n";
        let parsed = parsemeta("META-INF/neoforge.mods.toml", body)
            .unwrap()
            .unwrap();
        assert_eq!(parsed.modloaders, vec![ModLoaderType::Neoforge]);
        assert_eq!(
            parsemeta("META-INF/mods.toml", body)
                .unwrap()
                .unwrap()
                .modloaders,
            vec![ModLoaderType::Forge]
        );
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
            modid: Some(String::from("com.test.testmod")),
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
            modid: Some(String::from("com.test.testmod")),
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
    pub fn old_forge_metadata_broken() -> anyhow::Result<()> {
        let mcmodinfo = r#"
          [{
            "modid": invalid string with no quotes,
            "name": "TestMod",
            "description": "test desc",
            "version": "1.0.0",
            "authors": ["TestAuthor1", "TestAuthor2"],
            "logoFile": "/test/logo"
          }]
        "#;

        let expected = Some(ModFileMetadata {
            modid: None,
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
            modid: Some(String::from("com.test.testmod")),
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
            modid: Some(String::from("com.test.testmod")),
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
            modid: Some(String::from("com.test.testmod")),
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
            modid: Some(String::from("com.test.testmod")),
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
            modid: Some(String::from("com.test.testmod")),
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
    pub fn fabric_author_contact_object_test() -> anyhow::Result<()> {
        let modjson = r#"{
  "schemaVersion": 1,
  "id": "testmod",
  "version": "1.0.0",
  "name": "Test Mod",
  "authors": [
    {
      "name": "Test Author",
      "contact": {
        "email": "test@example.com"
      }
    }
  ]
}
        "#;

        let returned = parsemeta("fabric.mod.json", modjson)?;
        assert!(returned.is_some());

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
            modid: Some(String::from("com.test.testmod")),
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

    #[test]
    pub fn xaero_minimap_mcmod() -> anyhow::Result<()> {
        let mcmodinfo = r#"
[
{
	"logoFile": "",
	"credits": "",
	"authorList": [
		"TestAuthor"
	],
	"updateUrl": "",
	"name": "TestMod",
	"description": "The most typical testmod.",
	"version": "23.4.0",
	"modid": "testmod",
	"mcversion": "1.12.2",
	"url": "",
	"screenshots": [],
	"dependencies": []
}
]
        "#;

        let expected = Some(ModFileMetadata {
            modid: Some(String::from("testmod")),
            name: Some(String::from("TestMod")),
            version: Some(String::from("23.4.0")),
            description: Some(String::from("The most typical testmod.")),
            authors: Some(String::from("TestAuthor")),
            logo_file: Some(String::from("")),
            modloaders: vec![ModLoaderType::Forge],
        });

        let returned = parsemeta("mcmod.info", mcmodinfo)?;

        assert_eq!(returned, expected);
        Ok(())
    }
}
