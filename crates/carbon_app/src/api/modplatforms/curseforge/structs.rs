use std::collections::HashMap;

use rspc::Type;
use serde::{Deserialize, Serialize};

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEFeaturedModsResponse {
    pub featured: Vec<FEMod>,
    pub popular: Vec<FEMod>,
    pub recently_updated: Vec<FEMod>,
}

impl From<crate::domain::modplatforms::curseforge::FeaturedModsResponse>
    for FEFeaturedModsResponse
{
    fn from(
        featured_mods_response: crate::domain::modplatforms::curseforge::FeaturedModsResponse,
    ) -> Self {
        Self {
            featured: featured_mods_response
                .featured
                .into_iter()
                .map(|mod_| mod_.into())
                .collect(),
            popular: featured_mods_response
                .popular
                .into_iter()
                .map(|mod_| mod_.into())
                .collect(),
            recently_updated: featured_mods_response
                .recently_updated
                .into_iter()
                .map(|mod_| mod_.into())
                .collect(),
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEFile {
    pub id: i32,
    pub game_id: i32,
    pub mod_id: i32,
    pub is_available: bool,
    pub display_name: String,
    pub file_name: String,
    pub release_type: FEFileReleaseType,
    pub file_status: FEFileStatus,
    pub hashes: Vec<FEFileHash>,
    pub file_date: String, // Consider using a datetime library for date-time representation
    pub file_length: u32,
    pub download_count: u32,
    pub download_url: Option<String>,
    pub game_versions: Vec<String>,
    pub sortable_game_versions: Vec<FESortableGameVersion>,
    pub dependencies: Vec<FEFileDependency>,
    pub expose_as_alternative: Option<bool>,
    pub parent_project_file_id: Option<i32>,
    pub alternate_file_id: Option<i32>,
    pub is_server_pack: Option<bool>,
    pub server_pack_file_id: Option<i32>,
    pub is_early_access_content: Option<bool>,
    pub early_access_end_date: Option<String>, // Consider using a datetime library for date-time representation
    pub file_fingerprint: String,
    pub modules: Vec<FEFileModule>,
}

impl From<crate::domain::modplatforms::curseforge::File> for FEFile {
    fn from(file: crate::domain::modplatforms::curseforge::File) -> Self {
        Self {
            id: file.id,
            game_id: file.game_id,
            mod_id: file.mod_id,
            is_available: file.is_available,
            display_name: file.display_name,
            file_name: file.file_name,
            release_type: file.release_type.into(),
            file_status: file.file_status.into(),
            hashes: file.hashes.into_iter().map(|hash| hash.into()).collect(),
            file_date: file.file_date,
            file_length: file.file_length,
            download_count: file.download_count,
            download_url: file.download_url,
            game_versions: file.game_versions,
            sortable_game_versions: file
                .sortable_game_versions
                .into_iter()
                .map(|version| version.into())
                .collect(),
            dependencies: file
                .dependencies
                .into_iter()
                .map(|dependency| dependency.into())
                .collect(),
            expose_as_alternative: file.expose_as_alternative,
            parent_project_file_id: file.parent_project_file_id,
            alternate_file_id: file.alternate_file_id,
            is_server_pack: file.is_server_pack,
            server_pack_file_id: file.server_pack_file_id,
            is_early_access_content: file.is_early_access_content,
            early_access_end_date: file.early_access_end_date,
            file_fingerprint: file.file_fingerprint.to_string(),
            modules: file
                .modules
                .into_iter()
                .map(|module| module.into())
                .collect(),
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEFileDependency {
    pub mod_id: i32,
    pub relation_type: FEFileRelationType,
}

impl From<crate::domain::modplatforms::curseforge::FileDependency> for FEFileDependency {
    fn from(dependency: crate::domain::modplatforms::curseforge::FileDependency) -> Self {
        Self {
            mod_id: dependency.mod_id,
            relation_type: dependency.relation_type.into(),
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEFileHash {
    pub value: String,
    pub algo: FEHashAlgo,
}

impl From<crate::domain::modplatforms::curseforge::FileHash> for FEFileHash {
    fn from(hash: crate::domain::modplatforms::curseforge::FileHash) -> Self {
        Self {
            value: hash.value,
            algo: hash.algo.into(),
        }
    }
}

#[derive(Type, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FEFileReleaseType {
    Stable,
    Beta,
    Alpha,
}

impl From<crate::domain::modplatforms::curseforge::FileReleaseType> for FEFileReleaseType {
    fn from(release_type: crate::domain::modplatforms::curseforge::FileReleaseType) -> Self {
        match release_type {
            crate::domain::modplatforms::curseforge::FileReleaseType::Stable => {
                FEFileReleaseType::Stable
            }
            crate::domain::modplatforms::curseforge::FileReleaseType::Beta => {
                FEFileReleaseType::Beta
            }
            crate::domain::modplatforms::curseforge::FileReleaseType::Alpha => {
                FEFileReleaseType::Alpha
            }
        }
    }
}

#[derive(Type, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FEFileStatus {
    Processing,
    ChangesRequired,
    UnderReview,
    Approved,
    Rejected,
    MalwareDetected,
    Deleted,
    Archived,
    Testing,
    Released,
    ReadyForReview,
    Deprecated,
    Baking,
    AwaitingPublishing,
    FailedPublishing,
}

impl From<crate::domain::modplatforms::curseforge::FileStatus> for FEFileStatus {
    fn from(file_status: crate::domain::modplatforms::curseforge::FileStatus) -> Self {
        match file_status {
            crate::domain::modplatforms::curseforge::FileStatus::Processing => {
                FEFileStatus::Processing
            }
            crate::domain::modplatforms::curseforge::FileStatus::ChangesRequired => {
                FEFileStatus::ChangesRequired
            }
            crate::domain::modplatforms::curseforge::FileStatus::UnderReview => {
                FEFileStatus::UnderReview
            }
            crate::domain::modplatforms::curseforge::FileStatus::Approved => FEFileStatus::Approved,
            crate::domain::modplatforms::curseforge::FileStatus::Rejected => FEFileStatus::Rejected,
            crate::domain::modplatforms::curseforge::FileStatus::MalwareDetected => {
                FEFileStatus::MalwareDetected
            }
            crate::domain::modplatforms::curseforge::FileStatus::Deleted => FEFileStatus::Deleted,
            crate::domain::modplatforms::curseforge::FileStatus::Archived => FEFileStatus::Archived,
            crate::domain::modplatforms::curseforge::FileStatus::Testing => FEFileStatus::Testing,
            crate::domain::modplatforms::curseforge::FileStatus::Released => FEFileStatus::Released,
            crate::domain::modplatforms::curseforge::FileStatus::ReadyForReview => {
                FEFileStatus::ReadyForReview
            }
            crate::domain::modplatforms::curseforge::FileStatus::Deprecated => {
                FEFileStatus::Deprecated
            }
            crate::domain::modplatforms::curseforge::FileStatus::Baking => FEFileStatus::Baking,
            crate::domain::modplatforms::curseforge::FileStatus::AwaitingPublishing => {
                FEFileStatus::AwaitingPublishing
            }
            crate::domain::modplatforms::curseforge::FileStatus::FailedPublishing => {
                FEFileStatus::FailedPublishing
            }
        }
    }
}

#[derive(Type, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FEFileRelationType {
    EmbeddedLibrary,
    OptionalDependency,
    RequiredDependency,
    Tool,
    Incompatible,
    Include,
}

impl From<crate::domain::modplatforms::curseforge::FileRelationType> for FEFileRelationType {
    fn from(relation_type: crate::domain::modplatforms::curseforge::FileRelationType) -> Self {
        match relation_type {
            crate::domain::modplatforms::curseforge::FileRelationType::EmbeddedLibrary => {
                FEFileRelationType::EmbeddedLibrary
            }
            crate::domain::modplatforms::curseforge::FileRelationType::OptionalDependency => {
                FEFileRelationType::OptionalDependency
            }
            crate::domain::modplatforms::curseforge::FileRelationType::RequiredDependency => {
                FEFileRelationType::RequiredDependency
            }
            crate::domain::modplatforms::curseforge::FileRelationType::Tool => {
                FEFileRelationType::Tool
            }
            crate::domain::modplatforms::curseforge::FileRelationType::Incompatible => {
                FEFileRelationType::Incompatible
            }
            crate::domain::modplatforms::curseforge::FileRelationType::Include => {
                FEFileRelationType::Include
            }
        }
    }
}

#[derive(Type, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FEHashAlgo {
    Sha1,
    Md5,
}

impl From<crate::domain::modplatforms::curseforge::HashAlgo> for FEHashAlgo {
    fn from(hash_algo: crate::domain::modplatforms::curseforge::HashAlgo) -> Self {
        match hash_algo {
            crate::domain::modplatforms::curseforge::HashAlgo::Sha1 => FEHashAlgo::Sha1,
            crate::domain::modplatforms::curseforge::HashAlgo::Md5 => FEHashAlgo::Md5,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEFileModule {
    pub name: String,
    pub fingerprint: String,
}

impl From<crate::domain::modplatforms::curseforge::FileModule> for FEFileModule {
    fn from(file_module: crate::domain::modplatforms::curseforge::FileModule) -> Self {
        FEFileModule {
            name: file_module.name,
            fingerprint: file_module.fingerprint.to_string(),
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEFingerprintFuzzyMatch {
    pub id: i32,
    pub file: FEFile,
    pub latest_files: Vec<FEFile>,
    pub fingerprints: Vec<String>,
}

impl From<crate::domain::modplatforms::curseforge::FingerprintFuzzyMatch>
    for FEFingerprintFuzzyMatch
{
    fn from(fuzzy_match: crate::domain::modplatforms::curseforge::FingerprintFuzzyMatch) -> Self {
        FEFingerprintFuzzyMatch {
            id: fuzzy_match.id,
            file: fuzzy_match.file.into(),
            latest_files: fuzzy_match
                .latest_files
                .into_iter()
                .map(|f| f.into())
                .collect(),
            fingerprints: fuzzy_match
                .fingerprints
                .into_iter()
                .map(|f| f.to_string())
                .collect(),
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEFingerprintFuzzyMatchResult {
    pub fuzzy_matches: Vec<FEFingerprintFuzzyMatch>,
}

impl From<crate::domain::modplatforms::curseforge::FingerprintFuzzyMatchResult>
    for FEFingerprintFuzzyMatchResult
{
    fn from(
        fuzzy_match_result: crate::domain::modplatforms::curseforge::FingerprintFuzzyMatchResult,
    ) -> Self {
        FEFingerprintFuzzyMatchResult {
            fuzzy_matches: fuzzy_match_result
                .fuzzy_matches
                .into_iter()
                .map(|f| f.into())
                .collect(),
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEFingerprintMatch {
    pub id: i32,
    pub file: FEFile,
    pub latest_files: Vec<FEFile>,
}

impl From<crate::domain::modplatforms::curseforge::FingerprintMatch> for FEFingerprintMatch {
    fn from(match_: crate::domain::modplatforms::curseforge::FingerprintMatch) -> Self {
        FEFingerprintMatch {
            id: match_.id,
            file: match_.file.into(),
            latest_files: match_.latest_files.into_iter().map(|f| f.into()).collect(),
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEFingerprintsMatchesResult {
    pub is_cache_built: bool,
    pub exact_matches: Vec<FEFingerprintMatch>,
    pub exact_fingerprints: Vec<String>,
    pub partial_matches: Vec<FEFingerprintMatch>,
    pub partial_match_fingerprints: HashMap<String, Vec<String>>,
    pub installed_fingerprints: Vec<String>,
    pub unmatched_fingerprints: Vec<String>,
}

impl From<crate::domain::modplatforms::curseforge::FingerprintsMatchesResult>
    for FEFingerprintsMatchesResult
{
    fn from(
        matches_result: crate::domain::modplatforms::curseforge::FingerprintsMatchesResult,
    ) -> Self {
        FEFingerprintsMatchesResult {
            is_cache_built: matches_result.is_cache_built,
            exact_matches: matches_result
                .exact_matches
                .into_iter()
                .map(|m| m.into())
                .collect(),
            exact_fingerprints: matches_result
                .exact_fingerprints
                .into_iter()
                .map(|f| f.to_string())
                .collect(),
            partial_matches: matches_result
                .partial_matches
                .into_iter()
                .map(|m| m.into())
                .collect(),
            partial_match_fingerprints: matches_result
                .partial_match_fingerprints
                .into_iter()
                .map(|(k, v)| (k, v.into_iter().map(|f| f.to_string()).collect()))
                .collect(),
            installed_fingerprints: matches_result
                .installed_fingerprints
                .into_iter()
                .map(|f| f.to_string())
                .collect(),
            unmatched_fingerprints: matches_result
                .unmatched_fingerprints
                .into_iter()
                .flat_map(|uf| uf.into_iter())
                .map(|f| f.to_string())
                .collect(),
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEFolderFingerprint {
    pub foldername: String,
    pub fingerprints: Vec<String>,
}

impl From<crate::domain::modplatforms::curseforge::FolderFingerprint> for FEFolderFingerprint {
    fn from(
        folder_fingerprint: crate::domain::modplatforms::curseforge::FolderFingerprint,
    ) -> Self {
        FEFolderFingerprint {
            foldername: folder_fingerprint.foldername,
            fingerprints: folder_fingerprint
                .fingerprints
                .into_iter()
                .map(|f| f.to_string())
                .collect(),
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEGame {
    pub id: i32,
    pub name: String,
    pub slug: String,
    pub date_modified: String, // date-time
    pub assets: FEGameAssets,
    pub status: FECoreStatus,
    pub api_status: FECoreApiStatus,
}

impl From<crate::domain::modplatforms::curseforge::Game> for FEGame {
    fn from(game: crate::domain::modplatforms::curseforge::Game) -> Self {
        FEGame {
            id: game.id,
            name: game.name,
            slug: game.slug,
            date_modified: game.date_modified,
            assets: game.assets.into(),
            status: game.status.into(),
            api_status: game.api_status.into(),
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEMinecraftGameVersion {
    pub id: i32,
    pub game_version_id: i32,
    pub version_string: String,
    pub jar_download_url: String,
    pub json_download_url: String,
    pub approved: bool,
    pub date_modified: String, // date-time
    pub game_version_type_id: i32,
    pub game_version_status: FEGameVersionStatus,
    pub game_version_type_status: FEGameVersionTypeStatus,
}

impl From<crate::domain::modplatforms::curseforge::MinecraftGameVersion>
    for FEMinecraftGameVersion
{
    fn from(
        minecraft_game_version: crate::domain::modplatforms::curseforge::MinecraftGameVersion,
    ) -> Self {
        FEMinecraftGameVersion {
            id: minecraft_game_version.id,
            game_version_id: minecraft_game_version.game_version_id,
            version_string: minecraft_game_version.version_string,
            jar_download_url: minecraft_game_version.jar_download_url,
            json_download_url: minecraft_game_version.json_download_url,
            approved: minecraft_game_version.approved,
            date_modified: minecraft_game_version.date_modified,
            game_version_type_id: minecraft_game_version.game_version_type_id,
            game_version_status: minecraft_game_version.game_version_status.into(),
            game_version_type_status: minecraft_game_version.game_version_type_status.into(),
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEMinecraftModLoaderIndex {
    pub name: String,
    pub game_version: String,
    pub latest: bool,
    pub recommended: bool,
    pub date_modified: String, // date-time
    pub mod_loader_type: FEModLoaderType,
}

impl From<crate::domain::modplatforms::curseforge::MinecraftModLoaderIndex>
    for FEMinecraftModLoaderIndex
{
    fn from(
        minecraft_mod_loader_index: crate::domain::modplatforms::curseforge::MinecraftModLoaderIndex,
    ) -> Self {
        FEMinecraftModLoaderIndex {
            name: minecraft_mod_loader_index.name,
            game_version: minecraft_mod_loader_index.game_version,
            latest: minecraft_mod_loader_index.latest,
            recommended: minecraft_mod_loader_index.recommended,
            date_modified: minecraft_mod_loader_index.date_modified,
            mod_loader_type: minecraft_mod_loader_index.mod_loader_type.into(),
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEMinecraftModLoaderVersion {
    pub id: i32,
    pub game_version_id: i32,
    pub minecraft_game_version_id: i32,
    pub forge_version: String,
    pub name: String,
    pub mod_loader_type: FEModLoaderType,
    pub download_url: String,
    pub filename: String,
    pub install_method: FEModLoaderInstallMethod,
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
    pub mod_loader_game_version_status: FEGameVersionStatus,
    pub mod_loader_game_version_type_status: FEGameVersionTypeStatus,
    pub mc_game_version_id: i32,
    pub mc_game_version_type_id: i32,
    pub mc_game_version_status: FEGameVersionStatus,
    pub mc_game_version_type_status: FEGameVersionTypeStatus,
    pub install_profile_json: String,
}

impl From<crate::domain::modplatforms::curseforge::MinecraftModLoaderVersion>
    for FEMinecraftModLoaderVersion
{
    fn from(
        minecraft_mod_loader_version: crate::domain::modplatforms::curseforge::MinecraftModLoaderVersion,
    ) -> Self {
        FEMinecraftModLoaderVersion {
            id: minecraft_mod_loader_version.id,
            game_version_id: minecraft_mod_loader_version.game_version_id,
            minecraft_game_version_id: minecraft_mod_loader_version.minecraft_game_version_id,
            forge_version: minecraft_mod_loader_version.forge_version,
            name: minecraft_mod_loader_version.name,
            mod_loader_type: minecraft_mod_loader_version.mod_loader_type.into(),
            download_url: minecraft_mod_loader_version.download_url,
            filename: minecraft_mod_loader_version.filename,
            install_method: minecraft_mod_loader_version.install_method.into(),
            latest: minecraft_mod_loader_version.latest,
            recommended: minecraft_mod_loader_version.recommended,
            approved: minecraft_mod_loader_version.approved,
            date_modified: minecraft_mod_loader_version.date_modified,
            maven_version_string: minecraft_mod_loader_version.maven_version_string,
            version_json: minecraft_mod_loader_version.version_json,
            libraries_install_location: minecraft_mod_loader_version.libraries_install_location,
            minecraft_version: minecraft_mod_loader_version.minecraft_version,
            additional_files_json: minecraft_mod_loader_version.additional_files_json,
            mod_loader_game_version_id: minecraft_mod_loader_version.mod_loader_game_version_id,
            mod_loader_game_version_type_id: minecraft_mod_loader_version
                .mod_loader_game_version_type_id,
            mod_loader_game_version_status: minecraft_mod_loader_version
                .mod_loader_game_version_status
                .into(),
            mod_loader_game_version_type_status: minecraft_mod_loader_version
                .mod_loader_game_version_type_status
                .into(),
            mc_game_version_id: minecraft_mod_loader_version.mc_game_version_id,
            mc_game_version_type_id: minecraft_mod_loader_version.mc_game_version_type_id,
            mc_game_version_status: minecraft_mod_loader_version.mc_game_version_status.into(),
            mc_game_version_type_status: minecraft_mod_loader_version
                .mc_game_version_type_status
                .into(),
            install_profile_json: minecraft_mod_loader_version.install_profile_json,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEMod {
    pub id: i32,
    pub game_id: i32,
    pub name: String,
    pub slug: String,
    pub links: FEModLinks,
    pub summary: String,
    pub status: FEModStatus,
    pub download_count: u32,
    pub is_featured: bool,
    pub primary_category_id: i32,
    pub categories: Vec<FECategory>,
    pub class_id: Option<i32>, // TODO: Add all options to enum and use it
    pub authors: Vec<FEModAuthor>,
    pub logo: FEModAsset,
    pub screenshots: Vec<FEModAsset>,
    pub main_file_id: i32,
    pub latest_files: Vec<FEFile>,
    pub latest_files_indexes: Vec<FEFileIndex>,
    pub date_created: String,  // date-time
    pub date_modified: String, // date-time
    pub date_released: String, // date-time
    pub allow_mod_distribution: Option<bool>,
    pub game_popularity_rank: i32,
    pub is_available: bool,
    pub thumbs_up_count: i32,
}

impl From<crate::domain::modplatforms::curseforge::Mod> for FEMod {
    fn from(minecraft_mod: crate::domain::modplatforms::curseforge::Mod) -> Self {
        FEMod {
            id: minecraft_mod.id,
            game_id: minecraft_mod.game_id,
            name: minecraft_mod.name,
            slug: minecraft_mod.slug,
            links: minecraft_mod.links.into(),
            summary: minecraft_mod.summary,
            status: minecraft_mod.status.into(),
            download_count: minecraft_mod.download_count,
            is_featured: minecraft_mod.is_featured,
            primary_category_id: minecraft_mod.primary_category_id,
            categories: minecraft_mod
                .categories
                .into_iter()
                .map(|c| c.into())
                .collect(),
            class_id: minecraft_mod.class_id,
            authors: minecraft_mod
                .authors
                .into_iter()
                .map(|a| a.into())
                .collect(),
            logo: minecraft_mod.logo.into(),
            screenshots: minecraft_mod
                .screenshots
                .into_iter()
                .map(|s| s.into())
                .collect(),
            main_file_id: minecraft_mod.main_file_id,
            latest_files: minecraft_mod
                .latest_files
                .into_iter()
                .map(|f| f.into())
                .collect(),
            latest_files_indexes: minecraft_mod
                .latest_files_indexes
                .into_iter()
                .map(|f| f.into())
                .collect(),
            date_created: minecraft_mod.date_created,
            date_modified: minecraft_mod.date_modified,
            date_released: minecraft_mod.date_released,
            allow_mod_distribution: minecraft_mod.allow_mod_distribution,
            game_popularity_rank: minecraft_mod.game_popularity_rank,
            is_available: minecraft_mod.is_available,
            thumbs_up_count: minecraft_mod.thumbs_up_count,
        }
    }
}

#[derive(Type, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FEClassId {
    Mods,
    Modpacks,
}

impl From<crate::domain::modplatforms::curseforge::ClassId> for FEClassId {
    fn from(class_id: crate::domain::modplatforms::curseforge::ClassId) -> Self {
        match class_id {
            crate::domain::modplatforms::curseforge::ClassId::Mods => FEClassId::Mods,
            crate::domain::modplatforms::curseforge::ClassId::Modpacks => FEClassId::Modpacks,
        }
    }
}

impl From<FEClassId> for crate::domain::modplatforms::curseforge::ClassId {
    fn from(class_id: FEClassId) -> Self {
        match class_id {
            FEClassId::Mods => crate::domain::modplatforms::curseforge::ClassId::Mods,
            FEClassId::Modpacks => crate::domain::modplatforms::curseforge::ClassId::Modpacks,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEModDependencies {
    pub id: i32,
    pub mod_id: i32,
    pub file_id: i32,
    pub file_dependency_id: i32,
    pub type_id: i32,
    pub dependency_type: FEDependencyType,
}

impl From<crate::domain::modplatforms::curseforge::ModDependencies> for FEModDependencies {
    fn from(
        minecraft_mod_dependencies: crate::domain::modplatforms::curseforge::ModDependencies,
    ) -> Self {
        FEModDependencies {
            id: minecraft_mod_dependencies.id,
            mod_id: minecraft_mod_dependencies.mod_id,
            file_id: minecraft_mod_dependencies.file_id,
            file_dependency_id: minecraft_mod_dependencies.file_dependency_id,
            type_id: minecraft_mod_dependencies.type_id,
            dependency_type: minecraft_mod_dependencies.dependency_type.into(),
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEModFileModule {
    pub folder_name: String,
    pub fingerprint: String,
}

impl From<crate::domain::modplatforms::curseforge::ModFileModule> for FEModFileModule {
    fn from(
        minecraft_mod_file_module: crate::domain::modplatforms::curseforge::ModFileModule,
    ) -> Self {
        FEModFileModule {
            folder_name: minecraft_mod_file_module.folder_name,
            fingerprint: minecraft_mod_file_module.fingerprint.to_string(),
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEModFileStat {
    pub mod_id: i32,
    pub file_id: i32,
    pub timestamp: String, // date-time
    pub total_downloads: u32,
    pub downloads: u32,
    pub update_count: i32,
}

impl From<crate::domain::modplatforms::curseforge::ModFileStat> for FEModFileStat {
    fn from(minecraft_mod_file_stat: crate::domain::modplatforms::curseforge::ModFileStat) -> Self {
        FEModFileStat {
            mod_id: minecraft_mod_file_stat.mod_id,
            file_id: minecraft_mod_file_stat.file_id,
            timestamp: minecraft_mod_file_stat.timestamp,
            total_downloads: minecraft_mod_file_stat.total_downloads,
            downloads: minecraft_mod_file_stat.downloads,
            update_count: minecraft_mod_file_stat.update_count,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEModFileVersion {
    pub id: i32,
    pub mod_id: i32,
    pub file_id: i32,
    pub game_version_id: i32,
    pub game_version: String,
}

impl From<crate::domain::modplatforms::curseforge::ModFileVersion> for FEModFileVersion {
    fn from(
        minecraft_mod_file_version: crate::domain::modplatforms::curseforge::ModFileVersion,
    ) -> Self {
        FEModFileVersion {
            id: minecraft_mod_file_version.id,
            mod_id: minecraft_mod_file_version.mod_id,
            file_id: minecraft_mod_file_version.file_id,
            game_version_id: minecraft_mod_file_version.game_version_id,
            game_version: minecraft_mod_file_version.game_version,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FESortableGameVersion {
    pub game_version_name: String,
    pub game_version_padded: String,
    pub game_version: String,
    pub game_version_release_date: String, // date-time
    pub game_version_type_id: Option<i32>,
}

impl From<crate::domain::modplatforms::curseforge::SortableGameVersion> for FESortableGameVersion {
    fn from(
        minecraft_sortable_game_version: crate::domain::modplatforms::curseforge::SortableGameVersion,
    ) -> Self {
        FESortableGameVersion {
            game_version_name: minecraft_sortable_game_version.game_version_name,
            game_version_padded: minecraft_sortable_game_version.game_version_padded,
            game_version: minecraft_sortable_game_version.game_version,
            game_version_release_date: minecraft_sortable_game_version.game_version_release_date,
            game_version_type_id: minecraft_sortable_game_version.game_version_type_id,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEGameAssets {
    pub game: String,
    pub logo: String,
}

impl From<crate::domain::modplatforms::curseforge::GameAssets> for FEGameAssets {
    fn from(minecraft_game_assets: crate::domain::modplatforms::curseforge::GameAssets) -> Self {
        FEGameAssets {
            game: minecraft_game_assets.game,
            logo: minecraft_game_assets.logo,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FECoreStatus {
    Draft,
    Test,
    PendingReview,
    Rejected,
    Approved,
    Live,
}

impl From<crate::domain::modplatforms::curseforge::CoreStatus> for FECoreStatus {
    fn from(minecraft_core_status: crate::domain::modplatforms::curseforge::CoreStatus) -> Self {
        match minecraft_core_status {
            crate::domain::modplatforms::curseforge::CoreStatus::Draft => FECoreStatus::Draft,
            crate::domain::modplatforms::curseforge::CoreStatus::Test => FECoreStatus::Test,
            crate::domain::modplatforms::curseforge::CoreStatus::PendingReview => {
                FECoreStatus::PendingReview
            }
            crate::domain::modplatforms::curseforge::CoreStatus::Rejected => FECoreStatus::Rejected,
            crate::domain::modplatforms::curseforge::CoreStatus::Approved => FECoreStatus::Approved,
            crate::domain::modplatforms::curseforge::CoreStatus::Live => FECoreStatus::Live,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FECoreApiStatus {
    Private,
    Public,
}

impl From<crate::domain::modplatforms::curseforge::CoreApiStatus> for FECoreApiStatus {
    fn from(
        minecraft_core_api_status: crate::domain::modplatforms::curseforge::CoreApiStatus,
    ) -> Self {
        match minecraft_core_api_status {
            crate::domain::modplatforms::curseforge::CoreApiStatus::Private => {
                FECoreApiStatus::Private
            }
            crate::domain::modplatforms::curseforge::CoreApiStatus::Public => {
                FECoreApiStatus::Public
            }
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FEGameVersionStatus {
    Approved,
    Deleted,
    New,
}

impl From<crate::domain::modplatforms::curseforge::GameVersionStatus> for FEGameVersionStatus {
    fn from(
        minecraft_game_version_status: crate::domain::modplatforms::curseforge::GameVersionStatus,
    ) -> Self {
        match minecraft_game_version_status {
            crate::domain::modplatforms::curseforge::GameVersionStatus::Approved => {
                FEGameVersionStatus::Approved
            }
            crate::domain::modplatforms::curseforge::GameVersionStatus::Deleted => {
                FEGameVersionStatus::Deleted
            }
            crate::domain::modplatforms::curseforge::GameVersionStatus::New => {
                FEGameVersionStatus::New
            }
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FEGameVersionTypeStatus {
    Normal,
    Deleted,
}

impl From<crate::domain::modplatforms::curseforge::GameVersionTypeStatus>
    for FEGameVersionTypeStatus
{
    fn from(
        minecraft_game_version_type_status: crate::domain::modplatforms::curseforge::GameVersionTypeStatus,
    ) -> Self {
        match minecraft_game_version_type_status {
            crate::domain::modplatforms::curseforge::GameVersionTypeStatus::Normal => {
                FEGameVersionTypeStatus::Normal
            }
            crate::domain::modplatforms::curseforge::GameVersionTypeStatus::Deleted => {
                FEGameVersionTypeStatus::Deleted
            }
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FEModLoaderType {
    Forge,
    Cauldron,
    LiteLoader,
    Fabric,
    Quilt,
}

impl From<crate::domain::modplatforms::curseforge::ModLoaderType> for FEModLoaderType {
    fn from(
        minecraft_mod_loader_type: crate::domain::modplatforms::curseforge::ModLoaderType,
    ) -> Self {
        match minecraft_mod_loader_type {
            crate::domain::modplatforms::curseforge::ModLoaderType::Forge => FEModLoaderType::Forge,
            crate::domain::modplatforms::curseforge::ModLoaderType::Cauldron => {
                FEModLoaderType::Cauldron
            }
            crate::domain::modplatforms::curseforge::ModLoaderType::LiteLoader => {
                FEModLoaderType::LiteLoader
            }
            crate::domain::modplatforms::curseforge::ModLoaderType::Fabric => {
                FEModLoaderType::Fabric
            }
            crate::domain::modplatforms::curseforge::ModLoaderType::Quilt => FEModLoaderType::Quilt,
        }
    }
}

impl From<FEModLoaderType> for crate::domain::modplatforms::curseforge::ModLoaderType {
    fn from(
        minecraft_mod_loader_type: FEModLoaderType,
    ) -> crate::domain::modplatforms::curseforge::ModLoaderType {
        match minecraft_mod_loader_type {
            FEModLoaderType::Forge => crate::domain::modplatforms::curseforge::ModLoaderType::Forge,
            FEModLoaderType::Cauldron => {
                crate::domain::modplatforms::curseforge::ModLoaderType::Cauldron
            }
            FEModLoaderType::LiteLoader => {
                crate::domain::modplatforms::curseforge::ModLoaderType::LiteLoader
            }
            FEModLoaderType::Fabric => {
                crate::domain::modplatforms::curseforge::ModLoaderType::Fabric
            }
            FEModLoaderType::Quilt => crate::domain::modplatforms::curseforge::ModLoaderType::Quilt,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FEModLoaderInstallMethod {
    ForgeInstaller,
    ForgeJarInstall,
    ForgeInstallerV2,
}

impl From<crate::domain::modplatforms::curseforge::ModLoaderInstallMethod>
    for FEModLoaderInstallMethod
{
    fn from(
        minecraft_mod_loader_install_method: crate::domain::modplatforms::curseforge::ModLoaderInstallMethod,
    ) -> Self {
        match minecraft_mod_loader_install_method {
            crate::domain::modplatforms::curseforge::ModLoaderInstallMethod::ForgeInstaller => {
                FEModLoaderInstallMethod::ForgeInstaller
            }
            crate::domain::modplatforms::curseforge::ModLoaderInstallMethod::ForgeJarInstall => {
                FEModLoaderInstallMethod::ForgeJarInstall
            }
            crate::domain::modplatforms::curseforge::ModLoaderInstallMethod::ForgeInstallerV2 => {
                FEModLoaderInstallMethod::ForgeInstallerV2
            }
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEModLinks {
    pub website_url: Option<String>,
    pub wiki_url: Option<String>,
    pub issues_url: Option<String>,
    pub source_url: Option<String>,
}

impl From<crate::domain::modplatforms::curseforge::ModLinks> for FEModLinks {
    fn from(minecraft_mod_links: crate::domain::modplatforms::curseforge::ModLinks) -> Self {
        FEModLinks {
            website_url: minecraft_mod_links.website_url,
            wiki_url: minecraft_mod_links.wiki_url,
            issues_url: minecraft_mod_links.issues_url,
            source_url: minecraft_mod_links.source_url,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FEModStatus {
    New,
    ChangesRequired,
    UnderSoftReview,
    Approved,
    Rejected,
    ChangesMade,
    Inactive,
    Abandoned,
    Deleted,
    UnderReview,
}

impl From<crate::domain::modplatforms::curseforge::ModStatus> for FEModStatus {
    fn from(minecraft_mod_status: crate::domain::modplatforms::curseforge::ModStatus) -> Self {
        match minecraft_mod_status {
            crate::domain::modplatforms::curseforge::ModStatus::New => FEModStatus::New,
            crate::domain::modplatforms::curseforge::ModStatus::ChangesRequired => {
                FEModStatus::ChangesRequired
            }
            crate::domain::modplatforms::curseforge::ModStatus::UnderSoftReview => {
                FEModStatus::UnderSoftReview
            }
            crate::domain::modplatforms::curseforge::ModStatus::Approved => FEModStatus::Approved,
            crate::domain::modplatforms::curseforge::ModStatus::Rejected => FEModStatus::Rejected,
            crate::domain::modplatforms::curseforge::ModStatus::ChangesMade => {
                FEModStatus::ChangesMade
            }
            crate::domain::modplatforms::curseforge::ModStatus::Inactive => FEModStatus::Inactive,
            crate::domain::modplatforms::curseforge::ModStatus::Abandoned => FEModStatus::Abandoned,
            crate::domain::modplatforms::curseforge::ModStatus::Deleted => FEModStatus::Deleted,
            crate::domain::modplatforms::curseforge::ModStatus::UnderReview => {
                FEModStatus::UnderReview
            }
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FECategory {
    pub id: i32,
    pub name: String,
    pub slug: String,
    pub url: String,
    pub icon_url: String,
    pub date_modified: String,
    pub is_class: Option<bool>,
    pub class_id: Option<i32>,
    pub parent_category_id: Option<i32>,
    pub display_index: Option<i32>,
}

impl From<crate::domain::modplatforms::curseforge::Category> for FECategory {
    fn from(minecraft_category: crate::domain::modplatforms::curseforge::Category) -> Self {
        FECategory {
            id: minecraft_category.id,
            name: minecraft_category.name,
            slug: minecraft_category.slug,
            url: minecraft_category.url,
            icon_url: minecraft_category.icon_url,
            date_modified: minecraft_category.date_modified,
            is_class: minecraft_category.is_class,
            class_id: minecraft_category.class_id,
            parent_category_id: minecraft_category.parent_category_id,
            display_index: minecraft_category.display_index,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEModAuthor {
    pub id: i32,
    pub name: String,
    pub url: String,
}

impl From<crate::domain::modplatforms::curseforge::ModAuthor> for FEModAuthor {
    fn from(minecraft_mod_author: crate::domain::modplatforms::curseforge::ModAuthor) -> Self {
        FEModAuthor {
            id: minecraft_mod_author.id,
            name: minecraft_mod_author.name,
            url: minecraft_mod_author.url,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEModAsset {
    pub id: i32,
    pub mod_id: i32,
    pub title: String,
    pub description: String,
    pub thumbnail_url: String,
    pub url: String,
}

impl From<crate::domain::modplatforms::curseforge::ModAsset> for FEModAsset {
    fn from(minecraft_mod_asset: crate::domain::modplatforms::curseforge::ModAsset) -> Self {
        FEModAsset {
            id: minecraft_mod_asset.id,
            mod_id: minecraft_mod_asset.mod_id,
            title: minecraft_mod_asset.title,
            description: minecraft_mod_asset.description,
            thumbnail_url: minecraft_mod_asset.thumbnail_url,
            url: minecraft_mod_asset.url,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEFileIndex {
    pub game_version: String,
    pub file_id: i32,
    pub filename: String,
    pub release_type: FEFileReleaseType,
    pub game_version_type_id: Option<i32>,
    pub mod_loader: Option<FEModLoaderType>,
}

impl From<crate::domain::modplatforms::curseforge::FileIndex> for FEFileIndex {
    fn from(minecraft_file_index: crate::domain::modplatforms::curseforge::FileIndex) -> Self {
        FEFileIndex {
            game_version: minecraft_file_index.game_version,
            file_id: minecraft_file_index.file_id,
            filename: minecraft_file_index.filename,
            release_type: minecraft_file_index.release_type.into(),
            game_version_type_id: minecraft_file_index.game_version_type_id,
            mod_loader: minecraft_file_index
                .mod_loader
                .map(|mod_loader| mod_loader.into()),
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FEDependencyType {
    EmbeddedLibrary,
    OptionalDependency,
    RequiredDependency,
    Tool,
    Incompatible,
    Include,
}

impl From<crate::domain::modplatforms::curseforge::DependencyType> for FEDependencyType {
    fn from(
        minecraft_dependency_type: crate::domain::modplatforms::curseforge::DependencyType,
    ) -> Self {
        match minecraft_dependency_type {
            crate::domain::modplatforms::curseforge::DependencyType::EmbeddedLibrary => {
                FEDependencyType::EmbeddedLibrary
            }
            crate::domain::modplatforms::curseforge::DependencyType::OptionalDependency => {
                FEDependencyType::OptionalDependency
            }
            crate::domain::modplatforms::curseforge::DependencyType::RequiredDependency => {
                FEDependencyType::RequiredDependency
            }
            crate::domain::modplatforms::curseforge::DependencyType::Tool => FEDependencyType::Tool,
            crate::domain::modplatforms::curseforge::DependencyType::Incompatible => {
                FEDependencyType::Incompatible
            }
            crate::domain::modplatforms::curseforge::DependencyType::Include => {
                FEDependencyType::Include
            }
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEPagination {
    pub index: i32,
    pub page_size: i32,
    pub result_count: i32,
    pub total_count: i32,
}

impl From<crate::domain::modplatforms::curseforge::Pagination> for FEPagination {
    fn from(minecraft_pagination: crate::domain::modplatforms::curseforge::Pagination) -> Self {
        FEPagination {
            index: minecraft_pagination.index,
            page_size: minecraft_pagination.page_size,
            result_count: minecraft_pagination.result_count,
            total_count: minecraft_pagination.total_count,
        }
    }
}
