use std::collections::HashMap;

use carbon_platforms::curseforge::ClassId;
use carbon_platforms::modrinth::project::ProjectType;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use specta::Type;
use strum_macros::EnumIter;

use super::FESearchAPI;
use super::curseforge::structs::{CFFECategory, CFFEClassId, CFFEMod};
use super::modrinth::structs::{MRFECategory, MRFEProjectSearchResult, MRFEProjectType};
use super::{curseforge, modrinth};

#[derive(Type, Debug, Deserialize, Serialize, Clone, EnumIter)]
#[serde(rename_all = "camelCase")]
pub enum FEUnifiedSearchType {
    Mod,
    Modpack,
    ResourcePack,
    Shader,
    World,
    Plugin,
    Datapack,
    Unknown,
}

impl ToString for FEUnifiedSearchType {
    fn to_string(&self) -> String {
        match self {
            FEUnifiedSearchType::Mod => "mod",
            FEUnifiedSearchType::Modpack => "modpack",
            FEUnifiedSearchType::ResourcePack => "resourcepack",
            FEUnifiedSearchType::Shader => "shader",
            FEUnifiedSearchType::World => "world",
            FEUnifiedSearchType::Plugin => "plugin",
            FEUnifiedSearchType::Datapack => "datapack",
            FEUnifiedSearchType::Unknown => "unknown",
        }
        .to_string()
    }
}

impl From<CFFEClassId> for FEUnifiedSearchType {
    fn from(value: CFFEClassId) -> Self {
        match value {
            CFFEClassId::Mods => FEUnifiedSearchType::Mod,
            CFFEClassId::Modpacks => FEUnifiedSearchType::Modpack,
            CFFEClassId::ResourcePacks => FEUnifiedSearchType::ResourcePack,
            CFFEClassId::Shaders => FEUnifiedSearchType::Shader,
            CFFEClassId::Worlds => FEUnifiedSearchType::World,
            CFFEClassId::BukkitPlugins => FEUnifiedSearchType::Plugin,
            CFFEClassId::Customizations => FEUnifiedSearchType::ResourcePack,
            CFFEClassId::Addons => FEUnifiedSearchType::ResourcePack,
            CFFEClassId::Datapacks => FEUnifiedSearchType::Datapack,
            CFFEClassId::Other(_) => FEUnifiedSearchType::Unknown,
        }
    }
}

impl From<FEUnifiedSearchType> for CFFEClassId {
    fn from(value: FEUnifiedSearchType) -> Self {
        match value {
            FEUnifiedSearchType::Mod => CFFEClassId::Mods,
            FEUnifiedSearchType::Modpack => CFFEClassId::Modpacks,
            FEUnifiedSearchType::ResourcePack => CFFEClassId::ResourcePacks,
            FEUnifiedSearchType::Shader => CFFEClassId::Shaders,
            FEUnifiedSearchType::World => CFFEClassId::Worlds,
            FEUnifiedSearchType::Plugin => CFFEClassId::BukkitPlugins,
            FEUnifiedSearchType::Datapack => CFFEClassId::Datapacks,
            FEUnifiedSearchType::Unknown => CFFEClassId::Other(0),
        }
    }
}

impl From<MRFEProjectType> for FEUnifiedSearchType {
    fn from(value: MRFEProjectType) -> Self {
        match value {
            MRFEProjectType::Mod => FEUnifiedSearchType::Mod,
            MRFEProjectType::Modpack => FEUnifiedSearchType::Modpack,
            MRFEProjectType::ResourcePack => FEUnifiedSearchType::ResourcePack,
            MRFEProjectType::Shader => FEUnifiedSearchType::Shader,
            MRFEProjectType::Plugin => FEUnifiedSearchType::Plugin,
            MRFEProjectType::DataPack => FEUnifiedSearchType::Datapack,
            MRFEProjectType::Unknown => FEUnifiedSearchType::Unknown,
        }
    }
}

impl From<FEUnifiedSearchType> for MRFEProjectType {
    fn from(value: FEUnifiedSearchType) -> Self {
        match value {
            FEUnifiedSearchType::Mod => MRFEProjectType::Mod,
            FEUnifiedSearchType::Modpack => MRFEProjectType::Modpack,
            FEUnifiedSearchType::ResourcePack => MRFEProjectType::ResourcePack,
            FEUnifiedSearchType::Shader => MRFEProjectType::Shader,
            FEUnifiedSearchType::Plugin => MRFEProjectType::Plugin,
            FEUnifiedSearchType::Datapack => MRFEProjectType::DataPack,
            FEUnifiedSearchType::World => MRFEProjectType::Unknown,
            FEUnifiedSearchType::Unknown => MRFEProjectType::Unknown,
        }
    }
}

impl From<ProjectType> for FEUnifiedSearchType {
    fn from(value: ProjectType) -> Self {
        match value {
            ProjectType::Mod => FEUnifiedSearchType::Mod,
            ProjectType::Modpack => FEUnifiedSearchType::Modpack,
            ProjectType::ResourcePack => FEUnifiedSearchType::ResourcePack,
            ProjectType::Shader => FEUnifiedSearchType::Shader,
            ProjectType::Plugin => FEUnifiedSearchType::Plugin,
            ProjectType::DataPack => FEUnifiedSearchType::Datapack,
            ProjectType::Unknown => FEUnifiedSearchType::Unknown,
        }
    }
}

impl From<FEUnifiedSearchType> for ProjectType {
    fn from(value: FEUnifiedSearchType) -> Self {
        match value {
            FEUnifiedSearchType::Mod => ProjectType::Mod,
            FEUnifiedSearchType::Modpack => ProjectType::Modpack,
            FEUnifiedSearchType::ResourcePack => ProjectType::ResourcePack,
            FEUnifiedSearchType::Shader => ProjectType::Shader,
            FEUnifiedSearchType::Plugin => ProjectType::Plugin,
            FEUnifiedSearchType::Datapack => ProjectType::DataPack,
            FEUnifiedSearchType::World => ProjectType::Unknown,
            FEUnifiedSearchType::Unknown => ProjectType::Unknown,
        }
    }
}

impl From<ClassId> for FEUnifiedSearchType {
    fn from(value: ClassId) -> Self {
        match value {
            ClassId::Mods => FEUnifiedSearchType::Mod,
            ClassId::Modpacks => FEUnifiedSearchType::Modpack,
            ClassId::ResourcePacks => FEUnifiedSearchType::ResourcePack,
            ClassId::Shaders => FEUnifiedSearchType::Shader,
            ClassId::Worlds => FEUnifiedSearchType::World,
            ClassId::BukkitPlugins => FEUnifiedSearchType::Plugin,
            ClassId::Customizations => FEUnifiedSearchType::ResourcePack,
            ClassId::Addons => FEUnifiedSearchType::ResourcePack,
            ClassId::Datapacks => FEUnifiedSearchType::Datapack,
            ClassId::Other(_) => FEUnifiedSearchType::Unknown,
        }
    }
}

impl From<FEUnifiedSearchType> for ClassId {
    fn from(value: FEUnifiedSearchType) -> Self {
        match value {
            FEUnifiedSearchType::Mod => ClassId::Mods,
            FEUnifiedSearchType::Modpack => ClassId::Modpacks,
            FEUnifiedSearchType::ResourcePack => ClassId::ResourcePacks,
            FEUnifiedSearchType::Shader => ClassId::Shaders,
            FEUnifiedSearchType::World => ClassId::Worlds,
            FEUnifiedSearchType::Plugin => ClassId::BukkitPlugins,
            FEUnifiedSearchType::Datapack => ClassId::Datapacks,
            FEUnifiedSearchType::Unknown => ClassId::Other(0),
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEUnifiedAuthor {
    pub name: String,
    pub avatar_url: Option<String>,
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEUnifiedSearchResult {
    pub title: String,
    pub description: String,
    pub image_url: Option<String>,
    pub high_res_image_url: Option<String>,
    pub downloads_count: u32,
    pub id: String,
    pub release_date: String,
    pub last_updated: String,
    pub platform: FEUnifiedPlatform,
    pub r#type: FEUnifiedSearchType,
    pub authors: Vec<FEUnifiedAuthor>,
    pub website_url: Option<String>,
    pub categories: Vec<FEUnifiedCategoryId>,
    pub screenshot_urls: Vec<String>,
    pub minecraft_versions: Vec<String>,
}

impl From<CFFEMod> for FEUnifiedSearchResult {
    fn from(value: CFFEMod) -> Self {
        FEUnifiedSearchResult {
            title: value.name,
            description: value.summary,
            image_url: value.logo.as_ref().map(|logo| logo.thumbnail_url.clone()),
            high_res_image_url: value.logo.as_ref().map(|logo| logo.url.clone()),
            id: value.id.to_string(),
            release_date: value.date_created,
            last_updated: value.date_modified.to_string(),
            downloads_count: value.download_count,
            platform: FEUnifiedPlatform::Curseforge,
            r#type: value
                .class_id
                .map(|id| id.into())
                .unwrap_or(FEUnifiedSearchType::Unknown),
            authors: value
                .authors
                .into_iter()
                .map(|author| FEUnifiedAuthor {
                    name: author.name,
                    avatar_url: author.avatar_url,
                })
                .collect(),
            website_url: value.links.website_url,
            categories: value
                .categories
                .into_iter()
                .map(|category| FEUnifiedCategoryId::Curseforge(category.id))
                .collect(),
            screenshot_urls: value
                .screenshots
                .into_iter()
                .map(|screenshot| screenshot.url)
                .collect(),
            minecraft_versions: {
                let mut all_versions: Vec<String> = value
                    .latest_files_indexes
                    .iter()
                    .map(|v| v.game_version.clone())
                    .collect();

                // all_versions.sort_by(|a, b| {
                //     // Parse versions with a custom comparator that handles Minecraft versioning
                //     let parse_version = |v: &str| -> (u32, u32, u32) {
                //         let parts: Vec<&str> = v.split('.').collect();
                //         let major = parts
                //             .get(0)
                //             .and_then(|s| s.parse::<u32>().ok())
                //             .unwrap_or(0);
                //         let minor = parts
                //             .get(1)
                //             .and_then(|s| s.parse::<u32>().ok())
                //             .unwrap_or(0);
                //         let patch = parts
                //             .get(2)
                //             .and_then(|s| s.parse::<u32>().ok())
                //             .unwrap_or(0);
                //         (major, minor, patch)
                //     };

                //     let a_version = parse_version(a);
                //     let b_version = parse_version(b);

                //     // Compare versions component by component
                //     a_version.cmp(&b_version)
                // });

                all_versions.dedup();
                all_versions
            },
        }
    }
}

impl From<MRFEProjectSearchResult> for FEUnifiedSearchResult {
    fn from(value: MRFEProjectSearchResult) -> Self {
        FEUnifiedSearchResult {
            title: value.title,
            description: value.description,
            image_url: value.icon_url.as_ref().map(|url| url.clone()),
            high_res_image_url: value.icon_url.map(|url| url),
            id: value.project_id,
            release_date: value.date_created,
            last_updated: value.date_modified,
            downloads_count: value.downloads,
            platform: FEUnifiedPlatform::Modrinth,
            r#type: value.project_type.clone().into(),
            authors: vec![FEUnifiedAuthor {
                name: value.author,
                avatar_url: None,
            }],
            website_url: Some(format!(
                "https://modrinth.com/{}/{}",
                serde_plain::to_string(&value.project_type)
                    .expect("Cannot fail as there is a default fallback"),
                value.slug
            )),
            categories: value
                .categories
                .unwrap_or_default()
                .into_iter()
                .map(|category| FEUnifiedCategoryId::Modrinth(category))
                .collect(),
            screenshot_urls: value.gallery.unwrap_or_default(),
            minecraft_versions: value.versions,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEUnifiedPagination {
    pub index: u32,
    pub page_size: u32,
    pub result_count: u32,
    pub total_count: u32,
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEUnifiedSearchResponse {
    pub data: Vec<FEUnifiedSearchResult>,
    pub pagination: Option<FEUnifiedPagination>,
}

impl FEUnifiedSearchResponse {
    pub fn merge(
        cf_response: FEUnifiedSearchResponse,
        mr_response: FEUnifiedSearchResponse,
    ) -> Self {
        Self {
            data: cf_response
                .data
                .into_iter()
                .chain(mr_response.data.into_iter())
                .collect(),
            pagination: None,
        }
    }
}

impl From<curseforge::responses::FEModSearchResponse> for FEUnifiedSearchResponse {
    fn from(value: curseforge::responses::FEModSearchResponse) -> Self {
        FEUnifiedSearchResponse {
            data: value
                .data
                .into_iter()
                .map(FEUnifiedSearchResult::from)
                .collect(),
            pagination: value.pagination.map(|pagination| FEUnifiedPagination {
                index: pagination.index as u32,
                page_size: pagination.page_size as u32,
                result_count: pagination.result_count as u32,
                total_count: pagination.total_count as u32,
            }),
        }
    }
}

impl From<modrinth::responses::MRFEProjectSearchResponse> for FEUnifiedSearchResponse {
    fn from(value: modrinth::responses::MRFEProjectSearchResponse) -> Self {
        let result_count = value.hits.len();
        FEUnifiedSearchResponse {
            data: value
                .hits
                .into_iter()
                .map(FEUnifiedSearchResult::from)
                .collect(),
            pagination: Some(FEUnifiedPagination {
                index: value.offset,
                page_size: value.limit,
                result_count: result_count as u32,
                total_count: value.total_hits,
            }),
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEUnifiedCategories {
    pub modrinth: HashMap<String, FEUnifiedCategory>,
    pub curseforge: HashMap<i32, FEUnifiedCategory>,
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type", content = "value")]
pub enum FEUnifiedCategoryIcon {
    Url(String),
    Embedded(String),
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FEUnifiedPlatform {
    Curseforge,
    Modrinth,
}

#[derive(Type, Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum FEUnifiedCategoryId {
    Curseforge(i32),
    Modrinth(String),
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEUnifiedCategory {
    pub platform: FEUnifiedPlatform,
    pub id: FEUnifiedCategoryId,
    pub name: Option<String>,
    pub icon: Option<FEUnifiedCategoryIcon>,
    pub project_type: FEUnifiedSearchType,
    pub parent_id: Option<String>,
}

impl From<carbon_platforms::modrinth::tag::Category> for FEUnifiedCategory {
    fn from(value: carbon_platforms::modrinth::tag::Category) -> Self {
        FEUnifiedCategory {
            platform: FEUnifiedPlatform::Modrinth,
            id: FEUnifiedCategoryId::Modrinth(value.name.clone()),
            name: Some(value.name),
            icon: value.icon.map(|icon| FEUnifiedCategoryIcon::Embedded(icon)),
            project_type: value.project_type.into(),
            parent_id: match value.header.as_str() {
                "categories" => None,
                value => Some(value.to_string()),
            },
        }
    }
}

impl From<carbon_platforms::curseforge::Category> for FEUnifiedCategory {
    fn from(value: carbon_platforms::curseforge::Category) -> Self {
        FEUnifiedCategory {
            platform: FEUnifiedPlatform::Curseforge,
            id: FEUnifiedCategoryId::Curseforge(value.id),
            name: Some(value.name),
            icon: value.icon_url.map(|url| FEUnifiedCategoryIcon::Url(url)),
            project_type: value
                .class_id
                .map(|id| id.into())
                .unwrap_or(FEUnifiedSearchType::Unknown),
            parent_id: value.parent_category_id.map(|id| id.to_string()),
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEUnifiedModLoaders {
    pub curseforge: Vec<FEUnifiedModLoaderType>,
    pub modrinth: Vec<FEUnifiedModLoaderType>,
}

#[derive(
    Type,
    Debug,
    serde_enum_str::Deserialize_enum_str,
    serde_enum_str::Serialize_enum_str,
    PartialEq,
    Eq,
    Clone,
    strum_macros::EnumIter,
)]
#[serde(rename_all = "lowercase")]
pub enum FEUnifiedModLoaderType {
    // all
    Forge,
    NeoForge,
    Fabric,
    Quilt,
    LiteLoader,

    // curseforge
    Cauldron,

    // modrinth
    Bukkit,
    Bungeecord,
    Canvas,
    Datapack,
    Folia,
    Iris,
    Minecraft,
    Modloader,
    Optifine,
    Paper,
    Purpur,
    Rift,
    Spigot,
    Sponge,
    Vanilla,
    Velocity,
    Waterfall,

    #[serde(other)]
    Unknown,
}

impl TryFrom<FEUnifiedModLoaderType> for curseforge::structs::CFFEModLoaderType {
    type Error = anyhow::Error;

    fn try_from(value: FEUnifiedModLoaderType) -> Result<Self, Self::Error> {
        match value {
            FEUnifiedModLoaderType::Forge => Ok(curseforge::structs::CFFEModLoaderType::Forge),

            FEUnifiedModLoaderType::NeoForge => {
                Ok(curseforge::structs::CFFEModLoaderType::Neoforge)
            }
            FEUnifiedModLoaderType::Fabric => Ok(curseforge::structs::CFFEModLoaderType::Fabric),
            FEUnifiedModLoaderType::Quilt => Ok(curseforge::structs::CFFEModLoaderType::Quilt),
            FEUnifiedModLoaderType::LiteLoader => {
                Ok(curseforge::structs::CFFEModLoaderType::LiteLoader)
            }
            FEUnifiedModLoaderType::Cauldron => {
                Ok(curseforge::structs::CFFEModLoaderType::Cauldron)
            }
            value => Err(anyhow::anyhow!(
                "Curseforge does not support the `{}` loader",
                value.to_string()
            )),
        }
    }
}

impl TryFrom<FEUnifiedModLoaderType> for modrinth::structs::MRFELoaderType {
    type Error = anyhow::Error;

    fn try_from(value: FEUnifiedModLoaderType) -> Result<Self, Self::Error> {
        match value {
            FEUnifiedModLoaderType::Forge => Ok(modrinth::structs::MRFELoaderType::Forge),
            FEUnifiedModLoaderType::NeoForge => Ok(modrinth::structs::MRFELoaderType::Neoforge),
            FEUnifiedModLoaderType::Fabric => Ok(modrinth::structs::MRFELoaderType::Fabric),
            FEUnifiedModLoaderType::Quilt => Ok(modrinth::structs::MRFELoaderType::Quilt),
            FEUnifiedModLoaderType::LiteLoader => Ok(modrinth::structs::MRFELoaderType::Liteloader),
            FEUnifiedModLoaderType::Bukkit => Ok(modrinth::structs::MRFELoaderType::Bukkit),
            FEUnifiedModLoaderType::Bungeecord => Ok(modrinth::structs::MRFELoaderType::Bungeecord),
            FEUnifiedModLoaderType::Canvas => Ok(modrinth::structs::MRFELoaderType::Canvas),
            FEUnifiedModLoaderType::Datapack => Ok(modrinth::structs::MRFELoaderType::Datapack),
            FEUnifiedModLoaderType::Folia => Ok(modrinth::structs::MRFELoaderType::Folia),
            FEUnifiedModLoaderType::Iris => Ok(modrinth::structs::MRFELoaderType::Iris),
            FEUnifiedModLoaderType::Minecraft => Ok(modrinth::structs::MRFELoaderType::Minecraft),
            FEUnifiedModLoaderType::Modloader => Ok(modrinth::structs::MRFELoaderType::Modloader),
            FEUnifiedModLoaderType::Optifine => Ok(modrinth::structs::MRFELoaderType::Optifine),
            FEUnifiedModLoaderType::Paper => Ok(modrinth::structs::MRFELoaderType::Paper),
            FEUnifiedModLoaderType::Purpur => Ok(modrinth::structs::MRFELoaderType::Purpur),
            FEUnifiedModLoaderType::Rift => Ok(modrinth::structs::MRFELoaderType::Rift),
            FEUnifiedModLoaderType::Spigot => Ok(modrinth::structs::MRFELoaderType::Spigot),
            FEUnifiedModLoaderType::Sponge => Ok(modrinth::structs::MRFELoaderType::Sponge),
            FEUnifiedModLoaderType::Vanilla => Ok(modrinth::structs::MRFELoaderType::Vanilla),
            FEUnifiedModLoaderType::Velocity => Ok(modrinth::structs::MRFELoaderType::Velocity),
            FEUnifiedModLoaderType::Waterfall => Ok(modrinth::structs::MRFELoaderType::Waterfall),
            FEUnifiedModLoaderType::Unknown => {
                Err(anyhow::anyhow!("Can't use unknown modloader type"))
            }
            FEUnifiedModLoaderType::Cauldron => Err(anyhow::anyhow!(
                "Modrinth does not support the `Cauldron` loader"
            )),
        }
    }
}

impl From<modrinth::structs::MRFELoaderType> for FEUnifiedModLoaderType {
    fn from(value: modrinth::structs::MRFELoaderType) -> Self {
        match value {
            modrinth::structs::MRFELoaderType::Forge => FEUnifiedModLoaderType::Forge,
            modrinth::structs::MRFELoaderType::Neoforge => FEUnifiedModLoaderType::NeoForge,
            modrinth::structs::MRFELoaderType::Fabric => FEUnifiedModLoaderType::Fabric,
            modrinth::structs::MRFELoaderType::Quilt => FEUnifiedModLoaderType::Quilt,
            modrinth::structs::MRFELoaderType::Liteloader => FEUnifiedModLoaderType::LiteLoader,
            modrinth::structs::MRFELoaderType::Bukkit => FEUnifiedModLoaderType::Bukkit,
            modrinth::structs::MRFELoaderType::Bungeecord => FEUnifiedModLoaderType::Bungeecord,
            modrinth::structs::MRFELoaderType::Canvas => FEUnifiedModLoaderType::Canvas,
            modrinth::structs::MRFELoaderType::Datapack => FEUnifiedModLoaderType::Datapack,
            modrinth::structs::MRFELoaderType::Folia => FEUnifiedModLoaderType::Folia,
            modrinth::structs::MRFELoaderType::Iris => FEUnifiedModLoaderType::Iris,
            modrinth::structs::MRFELoaderType::Minecraft => FEUnifiedModLoaderType::Minecraft,
            modrinth::structs::MRFELoaderType::Modloader => FEUnifiedModLoaderType::Modloader,
            modrinth::structs::MRFELoaderType::Optifine => FEUnifiedModLoaderType::Optifine,
            modrinth::structs::MRFELoaderType::Paper => FEUnifiedModLoaderType::Paper,
            modrinth::structs::MRFELoaderType::Purpur => FEUnifiedModLoaderType::Purpur,
            modrinth::structs::MRFELoaderType::Rift => FEUnifiedModLoaderType::Rift,
            modrinth::structs::MRFELoaderType::Spigot => FEUnifiedModLoaderType::Spigot,
            modrinth::structs::MRFELoaderType::Sponge => FEUnifiedModLoaderType::Sponge,
            modrinth::structs::MRFELoaderType::Vanilla => FEUnifiedModLoaderType::Vanilla,
            modrinth::structs::MRFELoaderType::Velocity => FEUnifiedModLoaderType::Velocity,
            modrinth::structs::MRFELoaderType::Waterfall => FEUnifiedModLoaderType::Waterfall,
            _ => FEUnifiedModLoaderType::Unknown,
        }
    }
}

impl From<curseforge::structs::CFFEModLoaderType> for FEUnifiedModLoaderType {
    fn from(value: curseforge::structs::CFFEModLoaderType) -> Self {
        match value {
            curseforge::structs::CFFEModLoaderType::Forge => FEUnifiedModLoaderType::Forge,
            curseforge::structs::CFFEModLoaderType::Neoforge => FEUnifiedModLoaderType::NeoForge,
            curseforge::structs::CFFEModLoaderType::Fabric => FEUnifiedModLoaderType::Fabric,
            curseforge::structs::CFFEModLoaderType::Quilt => FEUnifiedModLoaderType::Quilt,
            curseforge::structs::CFFEModLoaderType::LiteLoader => {
                FEUnifiedModLoaderType::LiteLoader
            }
            curseforge::structs::CFFEModLoaderType::Cauldron => FEUnifiedModLoaderType::Cauldron,
            curseforge::structs::CFFEModLoaderType::Unknown => FEUnifiedModLoaderType::Unknown,
        }
    }
}
