use std::collections::HashMap;

use rspc::Type;
use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CFFEFeaturedModsResponse {
    pub featured: Vec<CFFEMod>,
    pub popular: Vec<CFFEMod>,
    pub recently_updated: Vec<CFFEMod>,
}

impl From<crate::domain::modplatforms::curseforge::FeaturedModsResponse>
    for CFFEFeaturedModsResponse
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
pub struct CFFEFile {
    pub id: i32,
    pub game_id: i32,
    pub mod_id: i32,
    pub is_available: bool,
    pub display_name: String,
    pub file_name: String,
    pub release_type: CFFEFileReleaseType,
    pub file_status: CFFEFileStatus,
    pub hashes: Vec<CFFEFileHash>,
    pub file_date: String, // Consider using a datetime library for date-time representation
    pub file_length: u32,
    pub download_count: u32,
    pub download_url: Option<String>,
    pub game_versions: Vec<String>,
    pub sortable_game_versions: Vec<CFFESortableGameVersion>,
    pub dependencies: Vec<CFFEFileDependency>,
    pub expose_as_alternative: Option<bool>,
    pub parent_project_file_id: Option<i32>,
    pub alternate_file_id: Option<i32>,
    pub is_server_pack: Option<bool>,
    pub server_pack_file_id: Option<i32>,
    pub is_early_access_content: Option<bool>,
    pub early_access_end_date: Option<String>, // Consider using a datetime library for date-time representation
    pub file_fingerprint: String,
    pub modules: Vec<CFFEFileModule>,
}

impl From<crate::domain::modplatforms::curseforge::File> for CFFEFile {
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
pub struct CFFEFileDependency {
    pub mod_id: i32,
    pub relation_type: CFFEFileRelationType,
}

impl From<crate::domain::modplatforms::curseforge::FileDependency> for CFFEFileDependency {
    fn from(dependency: crate::domain::modplatforms::curseforge::FileDependency) -> Self {
        Self {
            mod_id: dependency.mod_id,
            relation_type: dependency.relation_type.into(),
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CFFEFileHash {
    pub value: String,
    pub algo: CFFEHashAlgo,
}

impl From<crate::domain::modplatforms::curseforge::FileHash> for CFFEFileHash {
    fn from(hash: crate::domain::modplatforms::curseforge::FileHash) -> Self {
        Self {
            value: hash.value,
            algo: hash.algo.into(),
        }
    }
}

#[derive(Type, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CFFEFileReleaseType {
    Stable,
    Beta,
    Alpha,
}

impl From<crate::domain::modplatforms::curseforge::FileReleaseType> for CFFEFileReleaseType {
    fn from(release_type: crate::domain::modplatforms::curseforge::FileReleaseType) -> Self {
        match release_type {
            crate::domain::modplatforms::curseforge::FileReleaseType::Stable => {
                CFFEFileReleaseType::Stable
            }
            crate::domain::modplatforms::curseforge::FileReleaseType::Beta => {
                CFFEFileReleaseType::Beta
            }
            crate::domain::modplatforms::curseforge::FileReleaseType::Alpha => {
                CFFEFileReleaseType::Alpha
            }
        }
    }
}

#[derive(Type, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CFFEFileStatus {
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

impl From<crate::domain::modplatforms::curseforge::FileStatus> for CFFEFileStatus {
    fn from(file_status: crate::domain::modplatforms::curseforge::FileStatus) -> Self {
        match file_status {
            crate::domain::modplatforms::curseforge::FileStatus::Processing => {
                CFFEFileStatus::Processing
            }
            crate::domain::modplatforms::curseforge::FileStatus::ChangesRequired => {
                CFFEFileStatus::ChangesRequired
            }
            crate::domain::modplatforms::curseforge::FileStatus::UnderReview => {
                CFFEFileStatus::UnderReview
            }
            crate::domain::modplatforms::curseforge::FileStatus::Approved => {
                CFFEFileStatus::Approved
            }
            crate::domain::modplatforms::curseforge::FileStatus::Rejected => {
                CFFEFileStatus::Rejected
            }
            crate::domain::modplatforms::curseforge::FileStatus::MalwareDetected => {
                CFFEFileStatus::MalwareDetected
            }
            crate::domain::modplatforms::curseforge::FileStatus::Deleted => CFFEFileStatus::Deleted,
            crate::domain::modplatforms::curseforge::FileStatus::Archived => {
                CFFEFileStatus::Archived
            }
            crate::domain::modplatforms::curseforge::FileStatus::Testing => CFFEFileStatus::Testing,
            crate::domain::modplatforms::curseforge::FileStatus::Released => {
                CFFEFileStatus::Released
            }
            crate::domain::modplatforms::curseforge::FileStatus::ReadyForReview => {
                CFFEFileStatus::ReadyForReview
            }
            crate::domain::modplatforms::curseforge::FileStatus::Deprecated => {
                CFFEFileStatus::Deprecated
            }
            crate::domain::modplatforms::curseforge::FileStatus::Baking => CFFEFileStatus::Baking,
            crate::domain::modplatforms::curseforge::FileStatus::AwaitingPublishing => {
                CFFEFileStatus::AwaitingPublishing
            }
            crate::domain::modplatforms::curseforge::FileStatus::FailedPublishing => {
                CFFEFileStatus::FailedPublishing
            }
        }
    }
}

#[derive(Type, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CFFEFileRelationType {
    EmbeddedLibrary,
    OptionalDependency,
    RequiredDependency,
    Tool,
    Incompatible,
    Include,
}

impl From<crate::domain::modplatforms::curseforge::FileRelationType> for CFFEFileRelationType {
    fn from(relation_type: crate::domain::modplatforms::curseforge::FileRelationType) -> Self {
        match relation_type {
            crate::domain::modplatforms::curseforge::FileRelationType::EmbeddedLibrary => {
                CFFEFileRelationType::EmbeddedLibrary
            }
            crate::domain::modplatforms::curseforge::FileRelationType::OptionalDependency => {
                CFFEFileRelationType::OptionalDependency
            }
            crate::domain::modplatforms::curseforge::FileRelationType::RequiredDependency => {
                CFFEFileRelationType::RequiredDependency
            }
            crate::domain::modplatforms::curseforge::FileRelationType::Tool => {
                CFFEFileRelationType::Tool
            }
            crate::domain::modplatforms::curseforge::FileRelationType::Incompatible => {
                CFFEFileRelationType::Incompatible
            }
            crate::domain::modplatforms::curseforge::FileRelationType::Include => {
                CFFEFileRelationType::Include
            }
        }
    }
}

#[derive(Type, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CFFEHashAlgo {
    Sha1,
    Md5,
}

impl From<crate::domain::modplatforms::curseforge::HashAlgo> for CFFEHashAlgo {
    fn from(hash_algo: crate::domain::modplatforms::curseforge::HashAlgo) -> Self {
        match hash_algo {
            crate::domain::modplatforms::curseforge::HashAlgo::Sha1 => CFFEHashAlgo::Sha1,
            crate::domain::modplatforms::curseforge::HashAlgo::Md5 => CFFEHashAlgo::Md5,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CFFEFileModule {
    pub name: String,
    pub fingerprint: String,
}

impl From<crate::domain::modplatforms::curseforge::FileModule> for CFFEFileModule {
    fn from(file_module: crate::domain::modplatforms::curseforge::FileModule) -> Self {
        CFFEFileModule {
            name: file_module.name,
            fingerprint: file_module.fingerprint.to_string(),
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CFFEFingerprintFuzzyMatch {
    pub id: i32,
    pub file: CFFEFile,
    pub latest_files: Vec<CFFEFile>,
    pub fingerprints: Vec<String>,
}

impl From<crate::domain::modplatforms::curseforge::FingerprintFuzzyMatch>
    for CFFEFingerprintFuzzyMatch
{
    fn from(fuzzy_match: crate::domain::modplatforms::curseforge::FingerprintFuzzyMatch) -> Self {
        CFFEFingerprintFuzzyMatch {
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
pub struct CFFEFingerprintFuzzyMatchResult {
    pub fuzzy_matches: Vec<CFFEFingerprintFuzzyMatch>,
}

impl From<crate::domain::modplatforms::curseforge::FingerprintFuzzyMatchResult>
    for CFFEFingerprintFuzzyMatchResult
{
    fn from(
        fuzzy_match_result: crate::domain::modplatforms::curseforge::FingerprintFuzzyMatchResult,
    ) -> Self {
        CFFEFingerprintFuzzyMatchResult {
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
pub struct CFFEFingerprintMatch {
    pub id: i32,
    pub file: CFFEFile,
    pub latest_files: Vec<CFFEFile>,
}

impl From<crate::domain::modplatforms::curseforge::FingerprintMatch> for CFFEFingerprintMatch {
    fn from(match_: crate::domain::modplatforms::curseforge::FingerprintMatch) -> Self {
        CFFEFingerprintMatch {
            id: match_.id,
            file: match_.file.into(),
            latest_files: match_.latest_files.into_iter().map(|f| f.into()).collect(),
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CFFEFingerprintsMatchesResult {
    pub is_cache_built: bool,
    pub exact_matches: Vec<CFFEFingerprintMatch>,
    pub exact_fingerprints: Vec<String>,
    pub partial_matches: Vec<CFFEFingerprintMatch>,
    pub partial_match_fingerprints: HashMap<String, Vec<String>>,
    pub installed_fingerprints: Vec<String>,
    pub unmatched_fingerprints: Vec<String>,
}

impl From<crate::domain::modplatforms::curseforge::FingerprintsMatchesResult>
    for CFFEFingerprintsMatchesResult
{
    fn from(
        matches_result: crate::domain::modplatforms::curseforge::FingerprintsMatchesResult,
    ) -> Self {
        CFFEFingerprintsMatchesResult {
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
pub struct CFFEFolderFingerprint {
    pub foldername: String,
    pub fingerprints: Vec<String>,
}

impl From<crate::domain::modplatforms::curseforge::FolderFingerprint> for CFFEFolderFingerprint {
    fn from(
        folder_fingerprint: crate::domain::modplatforms::curseforge::FolderFingerprint,
    ) -> Self {
        CFFEFolderFingerprint {
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
pub struct CFFEGame {
    pub id: i32,
    pub name: String,
    pub slug: String,
    pub date_modified: String, // date-time
    pub assets: CFFEGameAssets,
    pub status: CFFECoreStatus,
    pub api_status: CFFECoreApiStatus,
}

impl From<crate::domain::modplatforms::curseforge::Game> for CFFEGame {
    fn from(game: crate::domain::modplatforms::curseforge::Game) -> Self {
        CFFEGame {
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
pub struct CFFEMinecraftGameVersion {
    pub id: i32,
    pub game_version_id: i32,
    pub version_string: String,
    pub jar_download_url: String,
    pub json_download_url: String,
    pub approved: bool,
    pub date_modified: String, // date-time
    pub game_version_type_id: i32,
    pub game_version_status: CFFEGameVersionStatus,
    pub game_version_type_status: CFFEGameVersionTypeStatus,
}

impl From<crate::domain::modplatforms::curseforge::MinecraftGameVersion>
    for CFFEMinecraftGameVersion
{
    fn from(
        minecraft_game_version: crate::domain::modplatforms::curseforge::MinecraftGameVersion,
    ) -> Self {
        CFFEMinecraftGameVersion {
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
pub struct CFFEMinecraftModLoaderIndex {
    pub name: String,
    pub game_version: String,
    pub latest: bool,
    pub recommended: bool,
    pub date_modified: String, // date-time
    pub mod_loader_type: CFFEModLoaderType,
}

impl From<crate::domain::modplatforms::curseforge::MinecraftModLoaderIndex>
    for CFFEMinecraftModLoaderIndex
{
    fn from(
        minecraft_mod_loader_index: crate::domain::modplatforms::curseforge::MinecraftModLoaderIndex,
    ) -> Self {
        CFFEMinecraftModLoaderIndex {
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
pub struct CFFEMinecraftModLoaderVersion {
    pub id: i32,
    pub game_version_id: i32,
    pub minecraft_game_version_id: i32,
    pub forge_version: String,
    pub name: String,
    pub mod_loader_type: CFFEModLoaderType,
    pub download_url: String,
    pub filename: String,
    pub install_method: CFFEModLoaderInstallMethod,
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
    pub mod_loader_game_version_status: CFFEGameVersionStatus,
    pub mod_loader_game_version_type_status: CFFEGameVersionTypeStatus,
    pub mc_game_version_id: i32,
    pub mc_game_version_type_id: i32,
    pub mc_game_version_status: CFFEGameVersionStatus,
    pub mc_game_version_type_status: CFFEGameVersionTypeStatus,
    pub install_profile_json: String,
}

impl From<crate::domain::modplatforms::curseforge::MinecraftModLoaderVersion>
    for CFFEMinecraftModLoaderVersion
{
    fn from(
        minecraft_mod_loader_version: crate::domain::modplatforms::curseforge::MinecraftModLoaderVersion,
    ) -> Self {
        CFFEMinecraftModLoaderVersion {
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
pub struct CFFEMod {
    pub id: i32,
    pub game_id: i32,
    pub name: String,
    pub slug: String,
    pub links: CFFEModLinks,
    pub summary: String,
    pub status: CFFEModStatus,
    pub download_count: u32,
    pub is_featured: bool,
    pub primary_category_id: i32,
    pub categories: Vec<CFFECategory>,
    pub class_id: Option<i32>, // TODO: Add all options to enum and use it
    pub authors: Vec<CFFEModAuthor>,
    pub logo: CFFEModAsset,
    pub screenshots: Vec<CFFEModAsset>,
    pub main_file_id: i32,
    pub latest_files: Vec<CFFEFile>,
    pub latest_files_indexes: Vec<CFFEFileIndex>,
    pub date_created: String,  // date-time
    pub date_modified: String, // date-time
    pub date_released: String, // date-time
    pub allow_mod_distribution: Option<bool>,
    pub game_popularity_rank: i32,
    pub is_available: bool,
    pub thumbs_up_count: i32,
}

impl From<crate::domain::modplatforms::curseforge::Mod> for CFFEMod {
    fn from(minecraft_mod: crate::domain::modplatforms::curseforge::Mod) -> Self {
        CFFEMod {
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
pub enum CFFEClassId {
    Mods,
    Modpacks,
}

impl From<crate::domain::modplatforms::curseforge::ClassId> for CFFEClassId {
    fn from(class_id: crate::domain::modplatforms::curseforge::ClassId) -> Self {
        match class_id {
            crate::domain::modplatforms::curseforge::ClassId::Mods => CFFEClassId::Mods,
            crate::domain::modplatforms::curseforge::ClassId::Modpacks => CFFEClassId::Modpacks,
        }
    }
}

impl From<CFFEClassId> for crate::domain::modplatforms::curseforge::ClassId {
    fn from(class_id: CFFEClassId) -> Self {
        match class_id {
            CFFEClassId::Mods => crate::domain::modplatforms::curseforge::ClassId::Mods,
            CFFEClassId::Modpacks => crate::domain::modplatforms::curseforge::ClassId::Modpacks,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CFFEModDependencies {
    pub id: i32,
    pub mod_id: i32,
    pub file_id: i32,
    pub file_dependency_id: i32,
    pub type_id: i32,
    pub dependency_type: CFFEDependencyType,
}

impl From<crate::domain::modplatforms::curseforge::ModDependencies> for CFFEModDependencies {
    fn from(
        minecraft_mod_dependencies: crate::domain::modplatforms::curseforge::ModDependencies,
    ) -> Self {
        CFFEModDependencies {
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
pub struct CFFEModFileModule {
    pub folder_name: String,
    pub fingerprint: String,
}

impl From<crate::domain::modplatforms::curseforge::ModFileModule> for CFFEModFileModule {
    fn from(
        minecraft_mod_file_module: crate::domain::modplatforms::curseforge::ModFileModule,
    ) -> Self {
        CFFEModFileModule {
            folder_name: minecraft_mod_file_module.folder_name,
            fingerprint: minecraft_mod_file_module.fingerprint.to_string(),
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CFFEModFileStat {
    pub mod_id: i32,
    pub file_id: i32,
    pub timestamp: String, // date-time
    pub total_downloads: u32,
    pub downloads: u32,
    pub update_count: i32,
}

impl From<crate::domain::modplatforms::curseforge::ModFileStat> for CFFEModFileStat {
    fn from(minecraft_mod_file_stat: crate::domain::modplatforms::curseforge::ModFileStat) -> Self {
        CFFEModFileStat {
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
pub struct CFFEModFileVersion {
    pub id: i32,
    pub mod_id: i32,
    pub file_id: i32,
    pub game_version_id: i32,
    pub game_version: String,
}

impl From<crate::domain::modplatforms::curseforge::ModFileVersion> for CFFEModFileVersion {
    fn from(
        minecraft_mod_file_version: crate::domain::modplatforms::curseforge::ModFileVersion,
    ) -> Self {
        CFFEModFileVersion {
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
pub struct CFFESortableGameVersion {
    pub game_version_name: String,
    pub game_version_padded: String,
    pub game_version: String,
    pub game_version_release_date: String, // date-time
    pub game_version_type_id: Option<i32>,
}

impl From<crate::domain::modplatforms::curseforge::SortableGameVersion>
    for CFFESortableGameVersion
{
    fn from(
        minecraft_sortable_game_version: crate::domain::modplatforms::curseforge::SortableGameVersion,
    ) -> Self {
        CFFESortableGameVersion {
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
pub struct CFFEGameAssets {
    pub game: String,
    pub logo: String,
}

impl From<crate::domain::modplatforms::curseforge::GameAssets> for CFFEGameAssets {
    fn from(minecraft_game_assets: crate::domain::modplatforms::curseforge::GameAssets) -> Self {
        CFFEGameAssets {
            game: minecraft_game_assets.game,
            logo: minecraft_game_assets.logo,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CFFECoreStatus {
    Draft,
    Test,
    PendingReview,
    Rejected,
    Approved,
    Live,
}

impl From<crate::domain::modplatforms::curseforge::CoreStatus> for CFFECoreStatus {
    fn from(minecraft_core_status: crate::domain::modplatforms::curseforge::CoreStatus) -> Self {
        match minecraft_core_status {
            crate::domain::modplatforms::curseforge::CoreStatus::Draft => CFFECoreStatus::Draft,
            crate::domain::modplatforms::curseforge::CoreStatus::Test => CFFECoreStatus::Test,
            crate::domain::modplatforms::curseforge::CoreStatus::PendingReview => {
                CFFECoreStatus::PendingReview
            }
            crate::domain::modplatforms::curseforge::CoreStatus::Rejected => {
                CFFECoreStatus::Rejected
            }
            crate::domain::modplatforms::curseforge::CoreStatus::Approved => {
                CFFECoreStatus::Approved
            }
            crate::domain::modplatforms::curseforge::CoreStatus::Live => CFFECoreStatus::Live,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CFFECoreApiStatus {
    Private,
    Public,
}

impl From<crate::domain::modplatforms::curseforge::CoreApiStatus> for CFFECoreApiStatus {
    fn from(
        minecraft_core_api_status: crate::domain::modplatforms::curseforge::CoreApiStatus,
    ) -> Self {
        match minecraft_core_api_status {
            crate::domain::modplatforms::curseforge::CoreApiStatus::Private => {
                CFFECoreApiStatus::Private
            }
            crate::domain::modplatforms::curseforge::CoreApiStatus::Public => {
                CFFECoreApiStatus::Public
            }
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CFFEGameVersionStatus {
    Approved,
    Deleted,
    New,
}

impl From<crate::domain::modplatforms::curseforge::GameVersionStatus> for CFFEGameVersionStatus {
    fn from(
        minecraft_game_version_status: crate::domain::modplatforms::curseforge::GameVersionStatus,
    ) -> Self {
        match minecraft_game_version_status {
            crate::domain::modplatforms::curseforge::GameVersionStatus::Approved => {
                CFFEGameVersionStatus::Approved
            }
            crate::domain::modplatforms::curseforge::GameVersionStatus::Deleted => {
                CFFEGameVersionStatus::Deleted
            }
            crate::domain::modplatforms::curseforge::GameVersionStatus::New => {
                CFFEGameVersionStatus::New
            }
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CFFEGameVersionTypeStatus {
    Normal,
    Deleted,
}

impl From<crate::domain::modplatforms::curseforge::GameVersionTypeStatus>
    for CFFEGameVersionTypeStatus
{
    fn from(
        minecraft_game_version_type_status: crate::domain::modplatforms::curseforge::GameVersionTypeStatus,
    ) -> Self {
        match minecraft_game_version_type_status {
            crate::domain::modplatforms::curseforge::GameVersionTypeStatus::Normal => {
                CFFEGameVersionTypeStatus::Normal
            }
            crate::domain::modplatforms::curseforge::GameVersionTypeStatus::Deleted => {
                CFFEGameVersionTypeStatus::Deleted
            }
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize, EnumIter)]
#[serde(rename_all = "camelCase")]
pub enum CFFEModLoaderType {
    Forge,
    Cauldron,
    LiteLoader,
    Fabric,
    Quilt,
}

impl From<crate::domain::modplatforms::curseforge::ModLoaderType> for CFFEModLoaderType {
    fn from(
        minecraft_mod_loader_type: crate::domain::modplatforms::curseforge::ModLoaderType,
    ) -> Self {
        match minecraft_mod_loader_type {
            crate::domain::modplatforms::curseforge::ModLoaderType::Forge => {
                CFFEModLoaderType::Forge
            }
            crate::domain::modplatforms::curseforge::ModLoaderType::Cauldron => {
                CFFEModLoaderType::Cauldron
            }
            crate::domain::modplatforms::curseforge::ModLoaderType::LiteLoader => {
                CFFEModLoaderType::LiteLoader
            }
            crate::domain::modplatforms::curseforge::ModLoaderType::Fabric => {
                CFFEModLoaderType::Fabric
            }
            crate::domain::modplatforms::curseforge::ModLoaderType::Quilt => {
                CFFEModLoaderType::Quilt
            }
        }
    }
}

impl From<CFFEModLoaderType> for crate::domain::modplatforms::curseforge::ModLoaderType {
    fn from(
        minecraft_mod_loader_type: CFFEModLoaderType,
    ) -> crate::domain::modplatforms::curseforge::ModLoaderType {
        match minecraft_mod_loader_type {
            CFFEModLoaderType::Forge => {
                crate::domain::modplatforms::curseforge::ModLoaderType::Forge
            }
            CFFEModLoaderType::Cauldron => {
                crate::domain::modplatforms::curseforge::ModLoaderType::Cauldron
            }
            CFFEModLoaderType::LiteLoader => {
                crate::domain::modplatforms::curseforge::ModLoaderType::LiteLoader
            }
            CFFEModLoaderType::Fabric => {
                crate::domain::modplatforms::curseforge::ModLoaderType::Fabric
            }
            CFFEModLoaderType::Quilt => {
                crate::domain::modplatforms::curseforge::ModLoaderType::Quilt
            }
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CFFEModLoaderInstallMethod {
    ForgeInstaller,
    ForgeJarInstall,
    ForgeInstallerV2,
}

impl From<crate::domain::modplatforms::curseforge::ModLoaderInstallMethod>
    for CFFEModLoaderInstallMethod
{
    fn from(
        minecraft_mod_loader_install_method: crate::domain::modplatforms::curseforge::ModLoaderInstallMethod,
    ) -> Self {
        match minecraft_mod_loader_install_method {
            crate::domain::modplatforms::curseforge::ModLoaderInstallMethod::ForgeInstaller => {
                CFFEModLoaderInstallMethod::ForgeInstaller
            }
            crate::domain::modplatforms::curseforge::ModLoaderInstallMethod::ForgeJarInstall => {
                CFFEModLoaderInstallMethod::ForgeJarInstall
            }
            crate::domain::modplatforms::curseforge::ModLoaderInstallMethod::ForgeInstallerV2 => {
                CFFEModLoaderInstallMethod::ForgeInstallerV2
            }
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CFFEModLinks {
    pub website_url: Option<String>,
    pub wiki_url: Option<String>,
    pub issues_url: Option<String>,
    pub source_url: Option<String>,
}

impl From<crate::domain::modplatforms::curseforge::ModLinks> for CFFEModLinks {
    fn from(minecraft_mod_links: crate::domain::modplatforms::curseforge::ModLinks) -> Self {
        CFFEModLinks {
            website_url: minecraft_mod_links.website_url,
            wiki_url: minecraft_mod_links.wiki_url,
            issues_url: minecraft_mod_links.issues_url,
            source_url: minecraft_mod_links.source_url,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CFFEModStatus {
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

impl From<crate::domain::modplatforms::curseforge::ModStatus> for CFFEModStatus {
    fn from(minecraft_mod_status: crate::domain::modplatforms::curseforge::ModStatus) -> Self {
        match minecraft_mod_status {
            crate::domain::modplatforms::curseforge::ModStatus::New => CFFEModStatus::New,
            crate::domain::modplatforms::curseforge::ModStatus::ChangesRequired => {
                CFFEModStatus::ChangesRequired
            }
            crate::domain::modplatforms::curseforge::ModStatus::UnderSoftReview => {
                CFFEModStatus::UnderSoftReview
            }
            crate::domain::modplatforms::curseforge::ModStatus::Approved => CFFEModStatus::Approved,
            crate::domain::modplatforms::curseforge::ModStatus::Rejected => CFFEModStatus::Rejected,
            crate::domain::modplatforms::curseforge::ModStatus::ChangesMade => {
                CFFEModStatus::ChangesMade
            }
            crate::domain::modplatforms::curseforge::ModStatus::Inactive => CFFEModStatus::Inactive,
            crate::domain::modplatforms::curseforge::ModStatus::Abandoned => {
                CFFEModStatus::Abandoned
            }
            crate::domain::modplatforms::curseforge::ModStatus::Deleted => CFFEModStatus::Deleted,
            crate::domain::modplatforms::curseforge::ModStatus::UnderReview => {
                CFFEModStatus::UnderReview
            }
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CFFECategory {
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

impl From<crate::domain::modplatforms::curseforge::Category> for CFFECategory {
    fn from(minecraft_category: crate::domain::modplatforms::curseforge::Category) -> Self {
        CFFECategory {
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
pub struct CFFEModAuthor {
    pub id: i32,
    pub name: String,
    pub url: String,
}

impl From<crate::domain::modplatforms::curseforge::ModAuthor> for CFFEModAuthor {
    fn from(minecraft_mod_author: crate::domain::modplatforms::curseforge::ModAuthor) -> Self {
        CFFEModAuthor {
            id: minecraft_mod_author.id,
            name: minecraft_mod_author.name,
            url: minecraft_mod_author.url,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CFFEModAsset {
    pub id: i32,
    pub mod_id: i32,
    pub title: String,
    pub description: String,
    pub thumbnail_url: String,
    pub url: String,
}

impl From<crate::domain::modplatforms::curseforge::ModAsset> for CFFEModAsset {
    fn from(minecraft_mod_asset: crate::domain::modplatforms::curseforge::ModAsset) -> Self {
        CFFEModAsset {
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
pub struct CFFEFileIndex {
    pub game_version: String,
    pub file_id: i32,
    pub filename: String,
    pub release_type: CFFEFileReleaseType,
    pub game_version_type_id: Option<i32>,
    pub mod_loader: Option<CFFEModLoaderType>,
}

impl From<crate::domain::modplatforms::curseforge::FileIndex> for CFFEFileIndex {
    fn from(minecraft_file_index: crate::domain::modplatforms::curseforge::FileIndex) -> Self {
        CFFEFileIndex {
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
pub enum CFFEDependencyType {
    EmbeddedLibrary,
    OptionalDependency,
    RequiredDependency,
    Tool,
    Incompatible,
    Include,
}

impl From<crate::domain::modplatforms::curseforge::DependencyType> for CFFEDependencyType {
    fn from(
        minecraft_dependency_type: crate::domain::modplatforms::curseforge::DependencyType,
    ) -> Self {
        match minecraft_dependency_type {
            crate::domain::modplatforms::curseforge::DependencyType::EmbeddedLibrary => {
                CFFEDependencyType::EmbeddedLibrary
            }
            crate::domain::modplatforms::curseforge::DependencyType::OptionalDependency => {
                CFFEDependencyType::OptionalDependency
            }
            crate::domain::modplatforms::curseforge::DependencyType::RequiredDependency => {
                CFFEDependencyType::RequiredDependency
            }
            crate::domain::modplatforms::curseforge::DependencyType::Tool => {
                CFFEDependencyType::Tool
            }
            crate::domain::modplatforms::curseforge::DependencyType::Incompatible => {
                CFFEDependencyType::Incompatible
            }
            crate::domain::modplatforms::curseforge::DependencyType::Include => {
                CFFEDependencyType::Include
            }
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CFFEPagination {
    pub index: i32,
    pub page_size: i32,
    pub result_count: i32,
    pub total_count: i32,
}

impl From<crate::domain::modplatforms::curseforge::Pagination> for CFFEPagination {
    fn from(minecraft_pagination: crate::domain::modplatforms::curseforge::Pagination) -> Self {
        CFFEPagination {
            index: minecraft_pagination.index,
            page_size: minecraft_pagination.page_size,
            result_count: minecraft_pagination.result_count,
            total_count: minecraft_pagination.total_count,
        }
    }
}
