use carbon_platforms::curseforge::{CurseForgeResponse, Mod};
use carbon_platforms::modrinth::project::{Project, ProjectType};
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
            project_type: FEUnifiedSearchType::Mod, // Assuming default project type is Mod
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
            icon: value.icon.map(FEUnifiedCategoryIcon::Url),
            project_type: FEUnifiedSearchType::Mod, // Assuming default project type is Mod
            parent_id: None,                        // Modrinth categories don't have parent IDs
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
    pub main_file_id: Option<String>,
}

impl From<ProjectSearchResponse> for FEUnifiedSearchResponse {
    fn from(value: ProjectSearchResponse) -> Self {
        todo!()
    }
}

impl From<CurseForgeResponse<Vec<Mod>>> for FEUnifiedSearchResponse {
    fn from(value: CurseForgeResponse<Vec<Mod>>) -> Self {
        todo!()
    }
}

impl From<CurseForgeResponse<Mod>> for FEUnifiedSearchResult {
    fn from(value: CurseForgeResponse<Mod>) -> Self {
        todo!()
    }
}

impl From<Project> for FEUnifiedSearchResult {
    fn from(value: Project) -> Self {
        todo!()
    }
}
