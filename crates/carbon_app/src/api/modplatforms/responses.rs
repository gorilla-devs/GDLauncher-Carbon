use carbon_platforms::curseforge::{CurseForgeResponse, Mod, ModLoaderType};
use carbon_platforms::modrinth::project::{Project, ProjectType};
use carbon_platforms::modrinth::search::ProjectSearchResult;
use carbon_platforms::modrinth::tag::LoaderType;
use carbon_platforms::{curseforge::ClassId, modrinth::search::ProjectSearchResponse};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::{collections::HashMap, ops::Deref};
use strum_macros::EnumIter;

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

#[derive(Type, Debug, Deserialize, Serialize, Clone)]
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

impl From<carbon_platforms::curseforge::Category> for FEUnifiedCategory {
    fn from(value: carbon_platforms::curseforge::Category) -> Self {
        FEUnifiedCategory {
            platform: FEUnifiedPlatform::Curseforge,
            id: FEUnifiedCategoryId::Curseforge(value.id),
            name: Some(value.name),
            icon: value.icon_url.map(FEUnifiedCategoryIcon::Url),
            project_type: value
                .class_id
                .map(|id| id.into())
                .unwrap_or(FEUnifiedSearchType::Unknown),
            parent_id: value.parent_category_id.map(|id| id.to_string()),
        }
    }
}

impl From<carbon_platforms::modrinth::tag::Category> for FEUnifiedCategory {
    fn from(value: carbon_platforms::modrinth::tag::Category) -> Self {
        FEUnifiedCategory {
            platform: FEUnifiedPlatform::Modrinth,
            id: FEUnifiedCategoryId::Modrinth(value.name.clone()),
            name: Some(value.name),
            icon: value.icon.map(FEUnifiedCategoryIcon::Embedded),
            project_type: value.project_type.into(),
            parent_id: None, // Modrinth categories don't have parent IDs
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEUnifiedModLoaders(pub Vec<FEUnifiedModLoaderType>);

impl Deref for FEUnifiedModLoaders {
    type Target = Vec<FEUnifiedModLoaderType>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
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

impl TryFrom<FEUnifiedModLoaderType> for ModLoaderType {
    type Error = anyhow::Error;

    fn try_from(value: FEUnifiedModLoaderType) -> Result<Self, Self::Error> {
        match value {
            FEUnifiedModLoaderType::Forge => Ok(ModLoaderType::Forge),

            FEUnifiedModLoaderType::NeoForge => Ok(ModLoaderType::NeoForge),
            FEUnifiedModLoaderType::Fabric => Ok(ModLoaderType::Fabric),
            FEUnifiedModLoaderType::Quilt => Ok(ModLoaderType::Quilt),
            FEUnifiedModLoaderType::LiteLoader => Ok(ModLoaderType::LiteLoader),
            FEUnifiedModLoaderType::Cauldron => Ok(ModLoaderType::Cauldron),
            value => Err(anyhow::anyhow!(
                "Curseforge does not support the `{}` loader",
                value.to_string()
            )),
        }
    }
}

impl TryFrom<FEUnifiedModLoaderType> for LoaderType {
    type Error = anyhow::Error;

    fn try_from(value: FEUnifiedModLoaderType) -> Result<Self, Self::Error> {
        match value {
            FEUnifiedModLoaderType::Forge => Ok(LoaderType::Forge),
            FEUnifiedModLoaderType::NeoForge => Ok(LoaderType::Neoforge),
            FEUnifiedModLoaderType::Fabric => Ok(LoaderType::Fabric),
            FEUnifiedModLoaderType::Quilt => Ok(LoaderType::Quilt),
            FEUnifiedModLoaderType::LiteLoader => Ok(LoaderType::Liteloader),
            FEUnifiedModLoaderType::Bukkit => Ok(LoaderType::Bukkit),
            FEUnifiedModLoaderType::Bungeecord => Ok(LoaderType::Bungeecord),
            FEUnifiedModLoaderType::Canvas => Ok(LoaderType::Canvas),
            FEUnifiedModLoaderType::Datapack => Ok(LoaderType::Datapack),
            FEUnifiedModLoaderType::Folia => Ok(LoaderType::Folia),
            FEUnifiedModLoaderType::Iris => Ok(LoaderType::Iris),
            FEUnifiedModLoaderType::Minecraft => Ok(LoaderType::Minecraft),
            FEUnifiedModLoaderType::Modloader => Ok(LoaderType::Modloader),
            FEUnifiedModLoaderType::Optifine => Ok(LoaderType::Optifine),
            FEUnifiedModLoaderType::Paper => Ok(LoaderType::Paper),
            FEUnifiedModLoaderType::Purpur => Ok(LoaderType::Purpur),
            FEUnifiedModLoaderType::Rift => Ok(LoaderType::Rift),
            FEUnifiedModLoaderType::Spigot => Ok(LoaderType::Spigot),
            FEUnifiedModLoaderType::Sponge => Ok(LoaderType::Sponge),
            FEUnifiedModLoaderType::Vanilla => Ok(LoaderType::Vanilla),
            FEUnifiedModLoaderType::Velocity => Ok(LoaderType::Velocity),
            FEUnifiedModLoaderType::Waterfall => Ok(LoaderType::Waterfall),
            FEUnifiedModLoaderType::Unknown => {
                Err(anyhow::anyhow!("Can't use unknown modloader type"))
            }
            FEUnifiedModLoaderType::Cauldron => Err(anyhow::anyhow!(
                "Modrinth does not support the `Cauldron` loader"
            )),
        }
    }
}

impl From<LoaderType> for FEUnifiedModLoaderType {
    fn from(value: LoaderType) -> Self {
        match value {
            LoaderType::Forge => FEUnifiedModLoaderType::Forge,
            LoaderType::Neoforge => FEUnifiedModLoaderType::NeoForge,
            LoaderType::Fabric => FEUnifiedModLoaderType::Fabric,
            LoaderType::Quilt => FEUnifiedModLoaderType::Quilt,
            LoaderType::Liteloader => FEUnifiedModLoaderType::LiteLoader,
            LoaderType::Bukkit => FEUnifiedModLoaderType::Bukkit,
            LoaderType::Bungeecord => FEUnifiedModLoaderType::Bungeecord,
            LoaderType::Canvas => FEUnifiedModLoaderType::Canvas,
            LoaderType::Datapack => FEUnifiedModLoaderType::Datapack,
            LoaderType::Folia => FEUnifiedModLoaderType::Folia,
            LoaderType::Iris => FEUnifiedModLoaderType::Iris,
            LoaderType::Minecraft => FEUnifiedModLoaderType::Minecraft,
            LoaderType::Modloader => FEUnifiedModLoaderType::Modloader,
            LoaderType::Optifine => FEUnifiedModLoaderType::Optifine,
            LoaderType::Paper => FEUnifiedModLoaderType::Paper,
            LoaderType::Purpur => FEUnifiedModLoaderType::Purpur,
            LoaderType::Rift => FEUnifiedModLoaderType::Rift,
            LoaderType::Spigot => FEUnifiedModLoaderType::Spigot,
            LoaderType::Sponge => FEUnifiedModLoaderType::Sponge,
            LoaderType::Vanilla => FEUnifiedModLoaderType::Vanilla,
            LoaderType::Velocity => FEUnifiedModLoaderType::Velocity,
            LoaderType::Waterfall => FEUnifiedModLoaderType::Waterfall,
            _ => FEUnifiedModLoaderType::Unknown,
        }
    }
}

impl From<ModLoaderType> for FEUnifiedModLoaderType {
    fn from(value: ModLoaderType) -> Self {
        match value {
            ModLoaderType::Forge => FEUnifiedModLoaderType::Forge,
            ModLoaderType::NeoForge => FEUnifiedModLoaderType::NeoForge,
            ModLoaderType::Fabric => FEUnifiedModLoaderType::Fabric,
            ModLoaderType::Quilt => FEUnifiedModLoaderType::Quilt,
            ModLoaderType::LiteLoader => FEUnifiedModLoaderType::LiteLoader,
            ModLoaderType::Cauldron => FEUnifiedModLoaderType::Cauldron,
            ModLoaderType::Other(_) => FEUnifiedModLoaderType::Unknown,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEUnifiedSearchResult {
    pub title: String,
    pub slug: String,
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
    pub versions: Option<Vec<String>>,
    pub main_file_id: Option<String>,
}

impl From<ProjectSearchResponse> for FEUnifiedSearchResponse {
    fn from(value: ProjectSearchResponse) -> Self {
        let result_count = value.hits.len();

        Self {
            data: value.hits.into_iter().map(|hit| hit.into()).collect(),
            pagination: Some(FEUnifiedPagination {
                index: value.offset,
                page_size: value.limit,
                result_count: result_count as u32,
                total_count: value.total_hits,
            }),
        }
    }
}

impl From<CurseForgeResponse<Vec<Mod>>> for FEUnifiedSearchResponse {
    fn from(value: CurseForgeResponse<Vec<Mod>>) -> Self {
        Self {
            data: value.data.into_iter().map(|m| m.into()).collect(),
            pagination: value.pagination.map(|pagination| FEUnifiedPagination {
                index: pagination.index as u32,
                page_size: pagination.page_size as u32,
                result_count: pagination.result_count as u32,
                total_count: pagination.total_count as u32,
            }),
        }
    }
}

impl From<CurseForgeResponse<Mod>> for FEUnifiedSearchResult {
    fn from(value: CurseForgeResponse<Mod>) -> Self {
        value.data.into()
    }
}

impl From<Mod> for FEUnifiedSearchResult {
    fn from(value: Mod) -> Self {
        FEUnifiedSearchResult {
            title: value.name,
            description: value.summary,
            slug: value.slug.clone(),
            main_file_id: Some(value.main_file_id.to_string()),
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
            versions: Some(
                value
                    .latest_files_indexes
                    .iter()
                    .map(|v| v.file_id.to_string())
                    .collect(),
            ),
        }
    }
}

impl From<Project> for FEUnifiedSearchResult {
    fn from(value: Project) -> Self {
        FEUnifiedSearchResult {
            title: value.title,
            description: value.description,
            slug: value.slug.clone(),
            main_file_id: value.versions.first().cloned(),
            image_url: value.icon_url.as_ref().map(|url| url.clone()),
            high_res_image_url: value.icon_url.map(|url| url),
            id: value.id,
            release_date: value.published.to_string(),
            last_updated: value.updated.to_string(),
            downloads_count: value.downloads,
            platform: FEUnifiedPlatform::Modrinth,
            r#type: value.project_type.clone().into(),
            authors: vec![],
            website_url: Some(format!(
                "https://modrinth.com/{}/{}",
                serde_plain::to_string(&value.project_type)
                    .expect("Cannot fail as there is a default fallback"),
                value.slug
            )),
            categories: value
                .categories
                .into_iter()
                .map(|category| FEUnifiedCategoryId::Modrinth(category))
                .collect(),
            screenshot_urls: value
                .gallery
                .iter()
                .map(|gallery| gallery.url.clone())
                .collect(),
            minecraft_versions: value.game_versions,
            versions: Some(value.versions),
        }
    }
}

impl FEUnifiedSearchResult {
    pub fn from_project_with_team(
        value: Project,
        team: Option<Vec<carbon_platforms::modrinth::user::TeamMember>>,
    ) -> Self {
        let authors = if let Some(team_members) = team {
            team_members
                .into_iter()
                .map(|member| FEUnifiedAuthor {
                    name: member.user.name.unwrap_or(member.user.username),
                    avatar_url: member.user.avatar_url,
                })
                .collect()
        } else {
            vec![]
        };

        FEUnifiedSearchResult {
            title: value.title,
            description: value.description,
            slug: value.slug.clone(),
            main_file_id: value.versions.first().cloned(),
            image_url: value.icon_url.as_ref().map(|url| url.clone()),
            high_res_image_url: value.icon_url.map(|url| url),
            id: value.id,
            release_date: value.published.to_string(),
            last_updated: value.updated.to_string(),
            downloads_count: value.downloads,
            platform: FEUnifiedPlatform::Modrinth,
            r#type: value.project_type.clone().into(),
            authors,
            website_url: Some(format!(
                "https://modrinth.com/{}/{}",
                serde_plain::to_string(&value.project_type)
                    .expect("Cannot fail as there is a default fallback"),
                value.slug
            )),
            categories: value
                .categories
                .into_iter()
                .map(|category| FEUnifiedCategoryId::Modrinth(category))
                .collect(),
            screenshot_urls: value
                .gallery
                .iter()
                .map(|gallery| gallery.url.clone())
                .collect(),
            minecraft_versions: value.game_versions,
            versions: Some(value.versions),
        }
    }
}

impl From<ProjectSearchResult> for FEUnifiedSearchResult {
    fn from(value: ProjectSearchResult) -> Self {
        FEUnifiedSearchResult {
            title: value.title,
            description: value.description,
            slug: value.slug.clone(),
            main_file_id: value.latest_version,
            image_url: value.icon_url.as_ref().map(|url| url.clone()),
            high_res_image_url: value.icon_url.map(|url| url),
            id: value.project_id,
            release_date: value.date_created.to_string(),
            last_updated: value.date_modified.to_string(),
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
            versions: None,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEUnifiedSearchResultWithDescription {
    pub full_description_body: String,
    #[serde(flatten)]
    pub result: FEUnifiedSearchResult,
}
