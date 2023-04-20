use reqwest_middleware::ClientWithMiddleware;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::collections::HashMap;
use url::Url;

use crate::managers::GDL_API_BASE;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeaturedModsResponse {
    pub featured: Vec<Mod>,
    pub popular: Vec<Mod>,
    pub recently_updated: Vec<Mod>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct File {
    pub id: i32,
    pub game_id: i32,
    pub mod_id: i32,
    pub is_available: bool,
    pub display_name: String,
    pub file_name: String,
    pub release_type: FileReleaseType,
    pub file_status: FileStatus,
    pub hashes: Vec<FileHash>,
    pub file_date: String, // Consider using a datetime library for date-time representation
    pub file_length: i64,
    pub download_count: i64,
    pub download_url: Option<String>,
    pub game_versions: Vec<String>,
    pub sortable_game_versions: Vec<SortableGameVersion>,
    pub dependencies: Vec<FileDependency>,
    pub expose_as_alternative: Option<bool>,
    pub parent_project_file_id: Option<i32>,
    pub alternate_file_id: Option<i32>,
    pub is_server_pack: Option<bool>,
    pub server_pack_file_id: Option<i32>,
    pub is_early_access_content: Option<bool>,
    pub early_access_end_date: Option<String>, // Consider using a datetime library for date-time representation
    pub file_fingerprint: i64,
    pub modules: Vec<FileModule>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDependency {
    pub mod_id: i32,
    pub relation_type: FileRelationType,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileHash {
    pub value: String,
    pub algo: HashAlgo,
}

#[derive(Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum FileReleaseType {
    Release = 1,
    Beta = 2,
    Alpha = 3,
}

#[derive(Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum FileStatus {
    Processing = 1,
    ChangesRequired = 2,
    UnderReview = 3,
    Approved = 4,
    Rejected = 5,
    MalwareDetected = 6,
    Deleted = 7,
    Archived = 8,
    Testing = 9,
    Released = 10,
    ReadyForReview = 11,
    Deprecated = 12,
    Baking = 13,
    AwaitingPublishing = 14,
    FailedPublishing = 15,
}

#[derive(Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum FileRelationType {
    EmbeddedLibrary = 1,
    OptionalDependency = 2,
    RequiredDependency = 3,
    Tool = 4,
    Incompatible = 5,
    Include = 6,
}

#[derive(Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum HashAlgo {
    Sha1 = 1,
    Md5 = 2,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileModule {
    pub name: String,
    pub fingerprint: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FingerprintFuzzyMatch {
    pub id: i32,
    pub file: File,
    pub latest_files: Vec<File>,
    pub fingerprints: Vec<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FingerprintFuzzyMatchResult {
    pub fuzzy_matches: Vec<FingerprintFuzzyMatch>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FingerprintMatch {
    pub id: i32,
    pub file: File,
    pub latest_files: Vec<File>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FingerprintsMatchesResult {
    pub is_cache_built: bool,
    pub exact_matches: Vec<FingerprintMatch>,
    pub exact_fingerprints: Vec<i32>,
    pub partial_matches: Vec<FingerprintMatch>,
    pub partial_match_fingerprints: HashMap<String, Vec<i32>>,
    pub installed_fingerprints: Vec<i32>,
    pub unmatched_fingerprints: Vec<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderFingerprint {
    pub foldername: String,
    pub fingerprints: Vec<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    pub id: i32,
    pub name: String,
    pub slug: String,
    pub date_modified: String, // date-time
    pub assets: GameAssets,
    pub status: CoreStatus,
    pub api_status: CoreApiStatus,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MinecraftGameVersion {
    pub id: i32,
    pub game_version_id: i32,
    pub version_string: String,
    pub jar_download_url: String,
    pub json_download_url: String,
    pub approved: bool,
    pub date_modified: String, // date-time
    pub game_version_type_id: i32,
    pub game_version_status: GameVersionStatus,
    pub game_version_type_status: GameVersionTypeStatus,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MinecraftModLoaderIndex {
    pub name: String,
    pub game_version: String,
    pub latest: bool,
    pub recommended: bool,
    pub date_modified: String, // date-time
    pub mod_loader_type: ModLoaderType,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MinecraftModLoaderVersion {
    pub id: i32,
    pub game_version_id: i32,
    pub minecraft_game_version_id: i32,
    pub forge_version: String,
    pub name: String,
    pub mod_loader_type: ModLoaderType,
    pub download_url: String,
    pub filename: String,
    pub install_method: ModLoaderInstallMethod,
    pub latest: bool,
    pub recommended: bool,
    pub approved: bool,
    pub date_modified: String, // date-time
    pub maven_version_string: String,
    pub version_json: String,
    pub libraries_install_location: String,
    pub minecraft_version: String,
    pub additional_files_json: String,
    pub mod_loader_game_version_id: i32,
    pub mod_loader_game_version_type_id: i32,
    pub mod_loader_game_version_status: GameVersionStatus,
    pub mod_loader_game_version_type_status: GameVersionTypeStatus,
    pub mc_game_version_id: i32,
    pub mc_game_version_type_id: i32,
    pub mc_game_version_status: GameVersionStatus,
    pub mc_game_version_type_status: GameVersionTypeStatus,
    pub install_profile_json: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mod {
    pub id: i32,
    pub game_id: i32,
    pub name: String,
    pub slug: String,
    pub links: ModLinks,
    pub summary: String,
    pub status: ModStatus,
    pub download_count: i64,
    pub is_featured: bool,
    pub primary_category_id: i32,
    pub categories: Vec<Category>,
    pub class_id: Option<i32>,
    pub authors: Vec<ModAuthor>,
    pub logo: ModAsset,
    pub screenshots: Vec<ModAsset>,
    pub main_file_id: i32,
    pub latest_files: Vec<File>,
    pub latest_files_indexes: Vec<FileIndex>,
    pub date_created: String,  // date-time
    pub date_modified: String, // date-time
    pub date_released: String, // date-time
    pub allow_mod_distribution: Option<bool>,
    pub game_popularity_rank: i32,
    pub is_available: bool,
    pub thumbs_up_count: i32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModDependencies {
    pub id: i32,
    pub mod_id: i32,
    pub file_id: i32,
    pub file_dependency_id: i32,
    pub type_id: i32,
    pub dependency_type: DependencyType,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModFile {
    pub id: i32,
    pub mod_id: i32,
    pub status: FileStatus,
    pub display_name: String,
    pub file_name: String,
    pub file_date: String, // date-time
    pub file_length: i64,
    pub release_type: FileReleaseType,
    pub is_available: bool,
    pub modules: Vec<ModFileModule>,
    pub game_version: Vec<String>,
    pub install_metadata: Option<String>,
    pub changelog: Option<String>,
    pub package_fingerprint: i64,
    pub file_dependencies: Vec<FileDependency>,
    pub is_alternate: bool,
    pub alternate_file_id: Option<i32>,
    pub download_url: String,
    pub is_available_in_manager: bool,
    pub project_status: ModStatus,
    pub render_cache_id: i32,
    pub file_mappings: Option<String>,
    pub expose_as_alternative: Option<bool>,
    pub package_hash: Option<String>,
    pub game_version_date_released: String, // date-time
    pub game_version_mapping_id: i32,
    pub game_version_id: i32,
    pub game_id: i32,
    pub is_primary: bool,
    pub game_version_type_id: i32,
    pub game_version_status: GameVersionStatus,
    pub game_version_type_status: GameVersionTypeStatus,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModFileModule {
    pub folder_name: String,
    pub fingerprint: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModFileStat {
    pub mod_id: i32,
    pub file_id: i32,
    pub timestamp: String, // date-time
    pub total_downloads: i64,
    pub downloads: i64,
    pub update_count: i32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModFileVersion {
    pub id: i32,
    pub mod_id: i32,
    pub file_id: i32,
    pub game_version_id: i32,
    pub game_version: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SortableGameVersion {
    pub game_version_name: String,
    pub game_version_padded: String,
    pub game_version: String,
    pub game_version_release_date: String, // date-time
    pub game_version_type_id: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameAssets {
    pub game: String,
    pub logo: String,
}

#[derive(Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum CoreStatus {
    Draft = 1,
    Test = 2,
    PendingReview = 3,
    Rejected = 4,
    Approved = 5,
    Live = 6,
}

#[derive(Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum CoreApiStatus {
    Private = 1,
    Public = 2,
}

#[derive(Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum GameVersionStatus {
    Approved = 1,
    Deleted = 2,
    New = 3,
}

#[derive(Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum GameVersionTypeStatus {
    Normal = 1,
    Deleted = 2,
}

#[derive(Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum ModLoaderType {
    Any = 0,
    Forge = 1,
    Cauldron = 2,
    LiteLoader = 3,
    Fabric = 4,
    Quilt = 5,
}

#[derive(Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum ModLoaderInstallMethod {
    ForgeInstaller = 1,
    ForgeJarInstall = 2,
    ForgeInstallerV2 = 3,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModLinks {
    pub website_url: Option<String>,
    pub wiki_url: Option<String>,
    pub issues_url: Option<String>,
    pub source_url: Option<String>,
}

#[derive(Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum ModStatus {
    New = 1,
    ChangesRequired = 2,
    UnderSoftReview = 3,
    Approved = 4,
    Rejected = 5,
    ChangesMade = 6,
    Inactive = 7,
    Abandoned = 8,
    Deleted = 9,
    UnderReview = 10,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: i32,
    pub name: String,
    pub slug: String,
    pub avatar_url: Option<String>,
    pub parent_category_id: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModAuthor {
    pub id: i32,
    pub name: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModAsset {
    pub id: i32,
    pub mod_id: i32,
    pub title: String,
    pub description: String,
    pub thumbnail_url: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileIndex {
    pub game_version: String,
    pub file_id: i32,
    pub filename: String,
    pub release_type: FileReleaseType,
    pub game_version_type_id: Option<i32>,
    pub mod_loader: Option<ModLoaderType>,
}

#[derive(Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum DependencyType {
    EmbeddedLibrary = 1,
    OptionalDependency = 2,
    RequiredDependency = 3,
    Tool = 4,
    Incompatible = 5,
    Include = 6,
}

// Parameters

#[derive(Debug)]
pub struct SearchParameters {
    pub game_id: i32,
    pub page: Option<i32>,
    pub search_query: Option<String>,
    pub mod_id: Option<i32>,
    pub category_id: Option<i32>,
    pub sort_by: Option<SortBy>,
    pub sort_direction: Option<SortDirection>,
}

impl SearchParameters {
    pub fn into_query_parameters(self) -> Vec<(&'static str, String)> {
        let mut params = Vec::new();
        params.push(("gameid", self.game_id.to_string()));

        if let Some(page) = self.page {
            params.push(("page", page.to_string()));
        }

        if let Some(ref search_query) = self.search_query {
            params.push(("search", search_query.clone()));
        }

        if let Some(mod_id) = self.mod_id {
            params.push(("id", mod_id.to_string()));
        }

        if let Some(category_id) = self.category_id {
            params.push(("cid", category_id.to_string()));
        }

        if let Some(ref sort_by) = self.sort_by {
            params.push(("sb", sort_by.to_string()));
        }

        if let Some(ref sort_direction) = self.sort_direction {
            params.push(("sd", sort_direction.to_string()));
        }

        params
    }
}

#[derive(Debug)]
pub enum SortBy {
    DateUpdated,
    DateCreated,
    Title,
    Author,
    Downloads,
    Subscribers,
    Favorites,
    Views,
}

impl ToString for SortBy {
    fn to_string(&self) -> String {
        match self {
            SortBy::DateUpdated => "updated",
            SortBy::DateCreated => "created",
            SortBy::Title => "title",
            SortBy::Author => "author",
            SortBy::Downloads => "downloads",
            SortBy::Subscribers => "subscribers",
            SortBy::Favorites => "favorites",
            SortBy::Views => "views",
        }
        .to_string()
    }
}

#[derive(Debug)]
pub enum SortDirection {
    Ascending,
    Descending,
}

impl ToString for SortDirection {
    fn to_string(&self) -> String {
        match self {
            SortDirection::Ascending => "asc",
            SortDirection::Descending => "desc",
        }
        .to_string()
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pagination {
    pub index: i32,
    pub page_size: i32,
    pub result_count: i32,
    pub total_count: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    pub data: Vec<Mod>,
    pub pagination: Pagination,
}

struct CurseForge {
    client: ClientWithMiddleware,
    base_url: Url,
}

impl CurseForge {
    pub fn new(client: reqwest_middleware::ClientWithMiddleware) -> Self {
        Self {
            client,
            base_url: format!("{GDL_API_BASE}/cf/").parse().unwrap(),
        }
    }

    pub async fn get_categories(&self) -> anyhow::Result<Vec<Category>> {
        let mut url = self.base_url.join("categories")?;
        url.set_query(Some("gameId=432"));

        let resp = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .json::<Vec<Category>>()
            .await?;
        Ok(resp)
    }

    pub async fn search(&self, search_params: SearchParameters) -> anyhow::Result<SearchResponse> {
        let mut url = self.base_url.join("mods/search")?;
        url.set_query(Some(&url_encode(search_params.into_query_parameters())));

        println!("url: {}", url.as_str());

        let resp = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .json::<SearchResponse>()
            .await?;
        Ok(resp)
    }
}

fn url_encode(into_query_parameters: Vec<(&str, String)>) -> String {
    let mut query = String::new();
    for (key, value) in into_query_parameters {
        query.push_str(&format!("{}={}&", key, value));
    }
    query.pop();

    query
}

#[cfg(test)]
mod test {
    #[tokio::test]
    async fn test_search_no_query() {
        use super::*;

        let client = reqwest::Client::builder().build().unwrap();
        let client = reqwest_middleware::ClientBuilder::new(client).build();
        let curseforge = CurseForge::new(client);

        let search_params = SearchParameters {
            game_id: 432,
            page: None,
            search_query: None,
            mod_id: None,
            category_id: None,
            sort_by: None,
            sort_direction: None,
        };

        let mods = curseforge.search(search_params).await.unwrap();
        assert!(!mods.data.is_empty());
    }

    #[tokio::test]
    async fn test_search_with_query() {
        use super::*;

        let client = reqwest::Client::builder().build().unwrap();
        let client = reqwest_middleware::ClientBuilder::new(client).build();
        let curseforge = CurseForge::new(client);

        let search_params = SearchParameters {
            game_id: 432,
            page: None,
            search_query: Some("jei".to_string()),
            mod_id: None,
            category_id: None,
            sort_by: None,
            sort_direction: None,
        };

        let mods = curseforge.search(search_params).await.unwrap();
        assert!(!mods.data.is_empty());
    }

    #[tokio::test]
    async fn test_search_with_query_and_page() {
        use super::*;

        let client = reqwest::Client::builder().build().unwrap();
        let client = reqwest_middleware::ClientBuilder::new(client).build();
        let curseforge = CurseForge::new(client);

        let search_params = SearchParameters {
            game_id: 432,
            page: Some(2),
            search_query: Some("jei".to_string()),
            mod_id: None,
            category_id: None,
            sort_by: None,
            sort_direction: None,
        };

        let mods = curseforge.search(search_params).await.unwrap();
        assert!(!mods.data.is_empty());
    }
}
