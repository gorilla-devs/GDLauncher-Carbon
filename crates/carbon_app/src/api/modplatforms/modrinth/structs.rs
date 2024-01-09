use std::collections::HashMap;

use rspc::Type;
use serde::{Deserialize, Serialize};

use crate::domain::modplatforms::modrinth::{
    project::{
        DonationLink, GalleryItem, License, ModeratorMessage, Project, ProjectStatus,
        ProjectSupportRange, ProjectType,
    },
    search::ProjectSearchResult,
    tag::{Category, Loader, LoaderType},
    user::{TeamMember, User, UserRole},
    version::{
        AdditionalFileType, Dependency, DependencyType, HashAlgorithm, Hashes,
        RequestedVersionStatus, Status, Version, VersionFile, VersionType,
    },
};

#[derive(Type, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MRFEProjectSupportRange {
    /// The mod is required on this side to function
    Required,
    /// The mod is not required on this side to function.
    /// However, functionality might be enhanced if it is present.
    Optional,
    /// The mod will not run on this side
    Unsupported,
    /// It is unknown if the project will run on this side
    Unknown,
}

impl From<ProjectSupportRange> for MRFEProjectSupportRange {
    fn from(value: ProjectSupportRange) -> Self {
        match value {
            ProjectSupportRange::Required => MRFEProjectSupportRange::Required,
            ProjectSupportRange::Unsupported => MRFEProjectSupportRange::Unsupported,
            ProjectSupportRange::Optional => MRFEProjectSupportRange::Optional,
            ProjectSupportRange::Unknown => MRFEProjectSupportRange::Unknown,
        }
    }
}

impl From<MRFEProjectSupportRange> for ProjectSupportRange {
    fn from(value: MRFEProjectSupportRange) -> Self {
        match value {
            MRFEProjectSupportRange::Required => ProjectSupportRange::Required,
            MRFEProjectSupportRange::Unsupported => ProjectSupportRange::Unsupported,
            MRFEProjectSupportRange::Optional => ProjectSupportRange::Optional,
            MRFEProjectSupportRange::Unknown => ProjectSupportRange::Unknown,
        }
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MRFEProjectType {
    /// WARNING: Can also be a plugin or data pack.
    /// You will have to read the loaders to get more specific information.
    Mod,
    Shader,
    Modpack,
    ResourcePack,
    Plugin,
    Project,
}

impl From<ProjectType> for MRFEProjectType {
    fn from(value: ProjectType) -> Self {
        match value {
            ProjectType::Mod => MRFEProjectType::Mod,
            ProjectType::Shader => MRFEProjectType::Shader,
            ProjectType::Modpack => MRFEProjectType::Modpack,
            ProjectType::ResourcePack => MRFEProjectType::ResourcePack,
            ProjectType::Plugin => MRFEProjectType::Plugin,
            ProjectType::Project => MRFEProjectType::Project,
        }
    }
}

impl From<MRFEProjectType> for ProjectType {
    fn from(value: MRFEProjectType) -> Self {
        match value {
            MRFEProjectType::Mod => ProjectType::Mod,
            MRFEProjectType::Shader => ProjectType::Shader,
            MRFEProjectType::Modpack => ProjectType::Modpack,
            MRFEProjectType::ResourcePack => ProjectType::ResourcePack,
            MRFEProjectType::Plugin => ProjectType::Plugin,
            MRFEProjectType::Project => ProjectType::Project,
        }
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct MRFEProjectSearchResult {
    /// The slug of a project used for vanity urls.
    pub slug: String,
    /// The title or name of the project
    pub title: String,
    /// A short description of the project
    pub description: String,
    /// A list of the categories that the project has
    pub categories: Option<Vec<String>>,
    /// The client side support of the project
    pub client_side: MRFEProjectSupportRange,
    /// The server side support of the project
    pub server_side: MRFEProjectSupportRange,
    /// The project type of the project
    pub project_type: MRFEProjectType,
    /// The total number of downloads of the project
    pub downloads: u32,
    /// The URL of the project's icon
    pub icon_url: Option<String>,
    /// The RGB color of the project, automatically generated form the project icon.
    pub color: Option<u32>,
    /// The ID of the project
    pub project_id: String,
    /// The username of the project's author
    pub author: String,
    /// A list of the categories that the project has which are not secondary
    pub display_categories: Option<Vec<String>>,
    /// A list of the minecraft versions supported by the project,
    pub versions: Vec<String>,
    /// The total number of users following the project
    pub follows: u32,
    /// The date the project was added to search
    pub date_created: String,
    /// The date the project was last modified
    pub date_modified: String,
    /// The latest version of minecraft that this project supports
    pub latest_version: Option<String>,
    /// the SPDX license of of a project
    pub license: String,
    /// All gallery images attached to the project
    pub gallery: Option<Vec<String>>,
    /// The featured gallery image of the project
    pub featured_gallery: Option<String>,
}

impl From<ProjectSearchResult> for MRFEProjectSearchResult {
    fn from(value: ProjectSearchResult) -> Self {
        MRFEProjectSearchResult {
            slug: value.slug,
            title: value.title,
            description: value.description,
            categories: value.categories,
            client_side: value.client_side.into(),
            server_side: value.server_side.into(),
            project_type: value.project_type.into(),
            downloads: value.downloads,
            icon_url: value.icon_url,
            color: value.color,
            project_id: value.project_id,
            author: value.author,
            display_categories: value.display_categories,
            versions: value.versions,
            follows: value.follows,
            date_created: value.date_created.to_rfc3339(),
            date_modified: value.date_modified.to_rfc3339(),
            latest_version: value.latest_version,
            license: value.license,
            gallery: value.gallery,
            featured_gallery: value.featured_gallery,
        }
    }
}

impl TryFrom<MRFEProjectSearchResult> for ProjectSearchResult {
    type Error = anyhow::Error;
    fn try_from(value: MRFEProjectSearchResult) -> Result<Self, Self::Error> {
        Ok(ProjectSearchResult {
            slug: value.slug,
            title: value.title,
            description: value.description,
            categories: value.categories,
            client_side: value.client_side.into(),
            server_side: value.server_side.into(),
            project_type: value.project_type.into(),
            downloads: value.downloads,
            icon_url: value.icon_url,
            color: value.color,
            project_id: value.project_id,
            author: value.author,
            display_categories: value.display_categories,
            versions: value.versions,
            follows: value.follows,
            date_created: value.date_created.parse()?,
            date_modified: value.date_modified.parse()?,
            latest_version: value.latest_version,
            license: value.license,
            gallery: value.gallery,
            featured_gallery: value.featured_gallery,
        })
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct MRFECategory {
    /// An SVG icon for the category
    pub icon: String,
    pub name: String,
    /// The project type this category is applicable to
    pub project_type: MRFEProjectType,
    /// The header under which the category should go
    pub header: String,
}

impl From<Category> for MRFECategory {
    fn from(value: Category) -> Self {
        MRFECategory {
            icon: value.icon,
            name: value.name,
            project_type: value.project_type.into(),
            header: value.header,
        }
    }
}

impl From<MRFECategory> for Category {
    fn from(value: MRFECategory) -> Self {
        Category {
            icon: value.icon,
            name: value.name,
            project_type: value.project_type.into(),
            header: value.header,
        }
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct MRFELoader {
    /// An SVG icon for the loader
    pub icon: String,
    pub name: MRFELoaderType,
    /// The project types that this loader can load
    pub supported_project_types: Vec<MRFEProjectType>,
}

impl From<Loader> for MRFELoader {
    fn from(value: Loader) -> Self {
        Self {
            icon: value.icon,
            name: value.name.into(),
            supported_project_types: value
                .supported_project_types
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}

impl From<MRFELoader> for Loader {
    fn from(value: MRFELoader) -> Self {
        Self {
            icon: value.icon,
            name: value.name.into(),
            supported_project_types: value
                .supported_project_types
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}

#[derive(
    Type,
    serde_enum_str::Deserialize_enum_str,
    serde_enum_str::Serialize_enum_str,
    Debug,
    PartialEq,
    Eq,
    Clone,
    strum_macros::EnumIter,
)]
#[serde(rename_all = "lowercase")]
pub enum MRFELoaderType {
    Bukkit,
    Bungeecord,
    Canvas,
    Datapack,
    Fabric,
    Folia,
    Forge,
    Neoforge,
    Iris,
    Liteloader,
    Minecraft,
    Modloader,
    Optifine,
    Paper,
    Purpur,
    Quilt,
    Rift,
    Spigot,
    Sponge,
    Vanilla,
    Velocity,
    Waterfall,
    Other,
}

impl From<LoaderType> for MRFELoaderType {
    fn from(value: LoaderType) -> Self {
        match value {
            LoaderType::Bukkit => MRFELoaderType::Bukkit,
            LoaderType::Bungeecord => MRFELoaderType::Bungeecord,
            LoaderType::Canvas => MRFELoaderType::Canvas,
            LoaderType::Datapack => MRFELoaderType::Datapack,
            LoaderType::Fabric => MRFELoaderType::Fabric,
            LoaderType::Folia => MRFELoaderType::Folia,
            LoaderType::Forge => MRFELoaderType::Forge,
            LoaderType::Neoforge => MRFELoaderType::Neoforge,
            LoaderType::Iris => MRFELoaderType::Iris,
            LoaderType::Liteloader => MRFELoaderType::Liteloader,
            LoaderType::Minecraft => MRFELoaderType::Minecraft,
            LoaderType::Modloader => MRFELoaderType::Modloader,
            LoaderType::Optifine => MRFELoaderType::Optifine,
            LoaderType::Paper => MRFELoaderType::Paper,
            LoaderType::Purpur => MRFELoaderType::Purpur,
            LoaderType::Quilt => MRFELoaderType::Quilt,
            LoaderType::Rift => MRFELoaderType::Rift,
            LoaderType::Spigot => MRFELoaderType::Spigot,
            LoaderType::Sponge => MRFELoaderType::Sponge,
            LoaderType::Vanilla => MRFELoaderType::Vanilla,
            LoaderType::Velocity => MRFELoaderType::Velocity,
            LoaderType::Waterfall => MRFELoaderType::Waterfall,
            LoaderType::Other(other) => MRFELoaderType::Other,
        }
    }
}

impl From<MRFELoaderType> for LoaderType {
    fn from(value: MRFELoaderType) -> Self {
        match value {
            MRFELoaderType::Bukkit => LoaderType::Bukkit,
            MRFELoaderType::Bungeecord => LoaderType::Bungeecord,
            MRFELoaderType::Canvas => LoaderType::Canvas,
            MRFELoaderType::Datapack => LoaderType::Datapack,
            MRFELoaderType::Fabric => LoaderType::Fabric,
            MRFELoaderType::Folia => LoaderType::Folia,
            MRFELoaderType::Forge => LoaderType::Forge,
            MRFELoaderType::Neoforge => LoaderType::Neoforge,
            MRFELoaderType::Iris => LoaderType::Iris,
            MRFELoaderType::Liteloader => LoaderType::Liteloader,
            MRFELoaderType::Minecraft => LoaderType::Minecraft,
            MRFELoaderType::Modloader => LoaderType::Modloader,
            MRFELoaderType::Optifine => LoaderType::Optifine,
            MRFELoaderType::Paper => LoaderType::Paper,
            MRFELoaderType::Purpur => LoaderType::Purpur,
            MRFELoaderType::Quilt => LoaderType::Quilt,
            MRFELoaderType::Rift => LoaderType::Rift,
            MRFELoaderType::Spigot => LoaderType::Spigot,
            MRFELoaderType::Sponge => LoaderType::Sponge,
            MRFELoaderType::Vanilla => LoaderType::Vanilla,
            MRFELoaderType::Velocity => LoaderType::Velocity,
            MRFELoaderType::Waterfall => LoaderType::Waterfall,
            MRFELoaderType::Other => LoaderType::Other("unknown".to_string()),
        }
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct MRFEVersion {
    pub name: String,
    /// The version number.
    /// Ideally, this will follow semantic versioning.
    pub version_number: String,
    pub changelog: Option<String>,
    pub dependencies: Vec<MRFEDependency>,
    pub game_versions: Vec<String>,
    /// The release channel for this version
    pub version_type: MRFEVersionType,
    pub loaders: Vec<String>,
    pub featured: bool,
    pub status: Option<MRFEStatus>,
    pub requested_status: Option<MRFERequestedVersionStatus>,
    pub id: String,
    /// The ID of the project this version is for
    pub project_id: String,
    /// The ID of the author who published this version
    pub author_id: String,
    pub date_published: String,
    pub downloads: u32,
    /// A list of files available for download
    pub files: Vec<MRFEVersionFile>,
}

impl From<Version> for MRFEVersion {
    fn from(value: Version) -> Self {
        MRFEVersion {
            name: value.name,
            version_number: value.version_number,
            changelog: value.changelog,
            dependencies: value.dependencies.into_iter().map(Into::into).collect(),
            game_versions: value.game_versions,
            version_type: value.version_type.into(),
            loaders: value.loaders,
            featured: value.featured,
            status: value.status.map(Into::into),
            requested_status: value.requested_status.map(Into::into),
            id: value.id,
            project_id: value.project_id,
            author_id: value.author_id,
            date_published: value.date_published.to_rfc3339(),
            downloads: value.downloads,
            files: value.files.into_iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<MRFEVersion> for Version {
    type Error = anyhow::Error;

    fn try_from(value: MRFEVersion) -> Result<Self, Self::Error> {
        #[allow(deprecated)]
        Ok(Version {
            name: value.name,
            version_number: value.version_number,
            changelog: value.changelog,
            dependencies: value.dependencies.into_iter().map(Into::into).collect(),
            game_versions: value.game_versions,
            version_type: value.version_type.into(),
            loaders: value.loaders,
            featured: value.featured,
            status: value.status.map(Into::into),
            requested_status: value.requested_status.map(Into::into),
            id: value.id,
            project_id: value.project_id,
            author_id: value.author_id,
            date_published: value.date_published.parse()?,
            downloads: value.downloads,
            changelog_url: None,
            files: value
                .files
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct MRFEVersionFile {
    pub hashes: MRFEHashes,
    pub url: String,
    pub filename: String,
    /// Whether the file is the primary file of its version.
    ///
    /// There can only be a maximum of one primary file per version.
    /// If there are no primary files specified, the first file can be taken as the primary file.
    pub primary: bool,
    /// The size of the file in bytes
    pub size: u32,
    /// The type of the additional file, used mainly for adding resource packs to datapacks
    pub file_type: Option<MRFEAdditionalFileType>,
}

impl From<VersionFile> for MRFEVersionFile {
    fn from(value: VersionFile) -> Self {
        MRFEVersionFile {
            hashes: value.hashes.into(),
            url: value.url,
            filename: value.filename,
            primary: value.primary,
            size: value.size,
            file_type: value.file_type.map(Into::into),
        }
    }
}

impl From<MRFEVersionFile> for VersionFile {
    fn from(value: MRFEVersionFile) -> Self {
        VersionFile {
            hashes: value.hashes.into(),
            url: value.url,
            filename: value.filename,
            primary: value.primary,
            size: value.size,
            file_type: value.file_type.map(Into::into),
        }
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct MRFEHashes {
    pub sha512: String,
    pub sha1: String,
    /// A map of other hashes that may have been provided
    #[serde(flatten)]
    pub others: HashMap<String, String>,
}

impl From<Hashes> for MRFEHashes {
    fn from(value: Hashes) -> Self {
        MRFEHashes {
            sha512: value.sha512,
            sha1: value.sha1,
            others: value.others,
        }
    }
}

impl From<MRFEHashes> for Hashes {
    fn from(value: MRFEHashes) -> Self {
        Hashes {
            sha512: value.sha512,
            sha1: value.sha1,
            others: value.others,
        }
    }
}

#[derive(Type, Serialize, Deserialize, Clone, Debug)]
pub struct MRFEDependency {
    pub version_id: Option<String>,
    pub project_id: Option<String>,
    pub file_name: Option<String>,
    pub dependency_type: MRFEDependencyType,
}

impl From<Dependency> for MRFEDependency {
    fn from(value: Dependency) -> Self {
        MRFEDependency {
            version_id: value.version_id,
            project_id: value.project_id,
            file_name: value.file_name,
            dependency_type: value.dependency_type.into(),
        }
    }
}

impl From<MRFEDependency> for Dependency {
    fn from(value: MRFEDependency) -> Self {
        Dependency {
            version_id: value.version_id,
            project_id: value.project_id,
            file_name: value.file_name,
            dependency_type: value.dependency_type.into(),
        }
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MRFEHashAlgorithm {
    SHA512,
    SHA1,
}

impl From<HashAlgorithm> for MRFEHashAlgorithm {
    fn from(value: HashAlgorithm) -> Self {
        match value {
            HashAlgorithm::SHA512 => MRFEHashAlgorithm::SHA512,
            HashAlgorithm::SHA1 => MRFEHashAlgorithm::SHA1,
        }
    }
}

impl From<MRFEHashAlgorithm> for HashAlgorithm {
    fn from(value: MRFEHashAlgorithm) -> Self {
        match value {
            MRFEHashAlgorithm::SHA512 => HashAlgorithm::SHA512,
            MRFEHashAlgorithm::SHA1 => HashAlgorithm::SHA1,
        }
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MRFEVersionType {
    Alpha,
    Beta,
    Release,
}

impl From<VersionType> for MRFEVersionType {
    fn from(value: VersionType) -> Self {
        match value {
            VersionType::Alpha => MRFEVersionType::Alpha,
            VersionType::Beta => MRFEVersionType::Beta,
            VersionType::Release => MRFEVersionType::Release,
        }
    }
}

impl From<MRFEVersionType> for VersionType {
    fn from(value: MRFEVersionType) -> Self {
        match value {
            MRFEVersionType::Alpha => VersionType::Alpha,
            MRFEVersionType::Beta => VersionType::Beta,
            MRFEVersionType::Release => VersionType::Release,
        }
    }
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MRFEDependencyType {
    Required,
    Optional,
    Incompatible,
    Embedded,
}

impl From<DependencyType> for MRFEDependencyType {
    fn from(value: DependencyType) -> Self {
        match value {
            DependencyType::Required => MRFEDependencyType::Required,
            DependencyType::Optional => MRFEDependencyType::Optional,
            DependencyType::Incompatible => MRFEDependencyType::Incompatible,
            DependencyType::Embedded => MRFEDependencyType::Embedded,
        }
    }
}

impl From<MRFEDependencyType> for DependencyType {
    fn from(value: MRFEDependencyType) -> Self {
        match value {
            MRFEDependencyType::Required => DependencyType::Required,
            MRFEDependencyType::Optional => DependencyType::Optional,
            MRFEDependencyType::Incompatible => DependencyType::Incompatible,
            MRFEDependencyType::Embedded => DependencyType::Embedded,
        }
    }
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MRFEStatus {
    Listed,
    Archived,
    Draft,
    Unlisted,
    Scheduled,
    Unknown,
}

impl From<Status> for MRFEStatus {
    fn from(value: Status) -> Self {
        match value {
            Status::Listed => MRFEStatus::Listed,
            Status::Archived => MRFEStatus::Archived,
            Status::Draft => MRFEStatus::Draft,
            Status::Unlisted => MRFEStatus::Unlisted,
            Status::Scheduled => MRFEStatus::Scheduled,
            Status::Unknown => MRFEStatus::Unknown,
        }
    }
}

impl From<MRFEStatus> for Status {
    fn from(value: MRFEStatus) -> Self {
        match value {
            MRFEStatus::Listed => Status::Listed,
            MRFEStatus::Archived => Status::Archived,
            MRFEStatus::Draft => Status::Draft,
            MRFEStatus::Unlisted => Status::Unlisted,
            MRFEStatus::Scheduled => Status::Scheduled,
            MRFEStatus::Unknown => Status::Unknown,
        }
    }
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MRFERequestedVersionStatus {
    Listed,
    Archived,
    Draft,
    Unlisted,
}

impl From<RequestedVersionStatus> for MRFERequestedVersionStatus {
    fn from(value: RequestedVersionStatus) -> Self {
        match value {
            RequestedVersionStatus::Listed => MRFERequestedVersionStatus::Listed,
            RequestedVersionStatus::Archived => MRFERequestedVersionStatus::Archived,
            RequestedVersionStatus::Draft => MRFERequestedVersionStatus::Draft,
            RequestedVersionStatus::Unlisted => MRFERequestedVersionStatus::Unlisted,
        }
    }
}

impl From<MRFERequestedVersionStatus> for RequestedVersionStatus {
    fn from(value: MRFERequestedVersionStatus) -> Self {
        match value {
            MRFERequestedVersionStatus::Listed => RequestedVersionStatus::Listed,
            MRFERequestedVersionStatus::Archived => RequestedVersionStatus::Archived,
            MRFERequestedVersionStatus::Draft => RequestedVersionStatus::Draft,
            MRFERequestedVersionStatus::Unlisted => RequestedVersionStatus::Unlisted,
        }
    }
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MRFEAdditionalFileType {
    RequiredResourcePack,
    OptionalResourcePack,
}

impl From<AdditionalFileType> for MRFEAdditionalFileType {
    fn from(value: AdditionalFileType) -> Self {
        match value {
            AdditionalFileType::RequiredResourcePack => {
                MRFEAdditionalFileType::RequiredResourcePack
            }
            AdditionalFileType::OptionalResourcePack => {
                MRFEAdditionalFileType::OptionalResourcePack
            }
        }
    }
}

impl From<MRFEAdditionalFileType> for AdditionalFileType {
    fn from(value: MRFEAdditionalFileType) -> Self {
        match value {
            MRFEAdditionalFileType::RequiredResourcePack => {
                AdditionalFileType::RequiredResourcePack
            }
            MRFEAdditionalFileType::OptionalResourcePack => {
                AdditionalFileType::OptionalResourcePack
            }
        }
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct MRFEProject {
    /// The project's slug, used for vanity URLs.
    /// This can change at any time, so use the [`Project::id`] for long term storage.
    pub slug: String,
    pub title: String,
    /// A short description of the project
    pub description: String,
    pub categories: Vec<String>,
    pub client_side: MRFEProjectSupportRange,
    pub server_side: MRFEProjectSupportRange,
    /// A long form description of the project
    pub body: String,
    /// A list of categories which are searchable but non-primary
    pub additional_categories: Vec<String>,
    /// A link to submit bugs or issues with the project
    pub issues_url: Option<String>,
    /// A link to the project's source code
    pub source_url: Option<String>,
    /// A link to the project's wiki page or other relevant information
    pub wiki_url: Option<String>,
    /// The project's Discord server invite
    pub discord_url: Option<String>,
    pub donation_urls: Vec<MRFEDonationLink>,
    pub project_type: MRFEProjectType,
    pub downloads: u32,
    pub icon_url: Option<String>,
    /// The RGB color of the project, automatically generated from the project icon
    pub color: Option<u32>,
    pub id: String,
    /// The ID of the team that has ownership of this project
    pub team: String,
    /// A link to the long description of the project (only present for old projects)
    pub moderator_message: Option<MRFEModeratorMessage>,
    pub published: String,
    pub updated: String,
    /// The date the project's status was set to approved or unlisted
    pub approved: Option<String>,
    pub followers: u32,
    pub status: MRFEProjectStatus,
    pub license: MRFELicense,
    /// A list of the version IDs of the project.
    /// This will only ever be empty if the project is a draft.
    pub versions: Vec<String>,
    /// A list of all of the game versions supported by the project
    pub game_versions: Vec<String>,
    /// A list of all of the loaders supported by the project
    pub loaders: Vec<String>,
    /// A list of images that have been uploaded to the project's gallery
    pub gallery: Vec<MRFEGalleryItem>,
}

impl From<Project> for MRFEProject {
    fn from(value: Project) -> Self {
        MRFEProject {
            slug: value.slug,
            title: value.title,
            description: value.description,
            categories: value.categories,
            client_side: value.client_side.into(),
            server_side: value.server_side.into(),
            body: value.body,
            additional_categories: value.additional_categories,
            issues_url: value.issues_url,
            source_url: value.source_url,
            wiki_url: value.wiki_url,
            discord_url: value.discord_url,
            donation_urls: value.donation_urls.into_iter().map(Into::into).collect(),
            project_type: value.project_type.into(),
            downloads: value.downloads,
            icon_url: value.icon_url,
            color: value.color,
            id: value.id,
            team: value.team,
            moderator_message: value.moderator_message.map(Into::into),
            published: value.published.to_rfc3339(),
            updated: value.updated.to_rfc3339(),
            approved: value.approved.map(|timestamp| timestamp.to_rfc3339()),
            followers: value.followers,
            status: value.status.into(),
            license: value.license.into(),
            versions: value.versions,
            game_versions: value.game_versions,
            loaders: value.loaders,
            gallery: value.gallery.into_iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<MRFEProject> for Project {
    type Error = anyhow::Error;
    fn try_from(value: MRFEProject) -> Result<Self, Self::Error> {
        #[allow(deprecated)]
        Ok(Project {
            slug: value.slug,
            title: value.title,
            description: value.description,
            categories: value.categories,
            client_side: value.client_side.into(),
            server_side: value.server_side.into(),
            body: value.body,
            additional_categories: value.additional_categories,
            issues_url: value.issues_url,
            source_url: value.source_url,
            wiki_url: value.wiki_url,
            discord_url: value.discord_url,
            donation_urls: value.donation_urls.into_iter().map(Into::into).collect(),
            project_type: value.project_type.into(),
            downloads: value.downloads,
            icon_url: value.icon_url,
            color: value.color,
            id: value.id,
            team: value.team,
            body_url: None,
            moderator_message: value.moderator_message.map(Into::into),
            published: value.published.parse()?,
            updated: value.updated.parse()?,
            approved: value
                .approved
                .map(|timestamp| timestamp.parse())
                .map_or(Ok(None), |timestamp| timestamp.map(Some))?,
            followers: value.followers,
            status: value.status.into(),
            license: value.license.try_into()?,
            versions: value.versions,
            game_versions: value.game_versions,
            loaders: value.loaders,
            gallery: value
                .gallery
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct MRFEModeratorMessage {
    pub message: String,
    /// The longer body of the message
    pub body: Option<String>,
}

impl From<ModeratorMessage> for MRFEModeratorMessage {
    fn from(value: ModeratorMessage) -> Self {
        MRFEModeratorMessage {
            message: value.message,
            body: value.body,
        }
    }
}

impl From<MRFEModeratorMessage> for ModeratorMessage {
    fn from(value: MRFEModeratorMessage) -> Self {
        ModeratorMessage {
            message: value.message,
            body: value.body,
        }
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct MRFELicense {
    /// The SPDX license ID of a project
    pub id: String,
    /// The license's long name
    pub name: String,
    /// The URL to this license
    pub url: Option<String>,
}

impl From<License> for MRFELicense {
    fn from(value: License) -> Self {
        MRFELicense {
            id: value.id,
            name: value.name,
            url: value.url,
        }
    }
}

impl From<MRFELicense> for License {
    fn from(value: MRFELicense) -> Self {
        License {
            id: value.id,
            name: value.name,
            url: value.url,
        }
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct MRFEDonationLink {
    /// The donation platform's ID
    pub id: String,
    pub platform: String,
    /// A link to the donation platform and user
    pub url: String,
}

impl From<DonationLink> for MRFEDonationLink {
    fn from(value: DonationLink) -> Self {
        MRFEDonationLink {
            id: value.id,
            platform: value.platform,
            url: value.url,
        }
    }
}

impl From<MRFEDonationLink> for DonationLink {
    fn from(value: MRFEDonationLink) -> Self {
        DonationLink {
            id: value.id,
            platform: value.platform,
            url: value.url,
        }
    }
}

/// An image that have been uploaded to a project's gallery
#[derive(Type, Serialize, Deserialize, Debug, Clone)]
pub struct MRFEGalleryItem {
    pub url: String,
    pub featured: bool,
    pub title: Option<String>,
    pub description: Option<String>,
    pub created: String,
    /// The order of the gallery image.
    /// Gallery images are sorted by this field and then alphabetically by title.
    pub ordering: Option<i32>,
}

impl From<GalleryItem> for MRFEGalleryItem {
    fn from(value: GalleryItem) -> Self {
        MRFEGalleryItem {
            url: value.url,
            featured: value.featured,
            title: value.title,
            description: value.description,
            created: value.created.to_rfc3339(),
            ordering: value.ordering,
        }
    }
}

impl TryFrom<MRFEGalleryItem> for GalleryItem {
    type Error = anyhow::Error;
    fn try_from(value: MRFEGalleryItem) -> Result<Self, Self::Error> {
        Ok(GalleryItem {
            url: value.url,
            featured: value.featured,
            title: value.title,
            description: value.description,
            created: value.created.parse()?,
            ordering: value.ordering,
        })
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MRFEProjectStatus {
    Approved,
    /// A moderator's message should be available on the project struct
    Rejected,
    Draft,
    /// The project has been approved and is publicly accessible, but will not show up in search results
    Unlisted,
    Archived,
    /// The project has been submitted for approval and is being reviewed
    Processing,
    Withheld,
    Unknown,
}

impl From<ProjectStatus> for MRFEProjectStatus {
    fn from(value: ProjectStatus) -> Self {
        match value {
            ProjectStatus::Approved => MRFEProjectStatus::Approved,
            ProjectStatus::Rejected => MRFEProjectStatus::Rejected,
            ProjectStatus::Draft => MRFEProjectStatus::Draft,
            ProjectStatus::Unlisted => MRFEProjectStatus::Unlisted,
            ProjectStatus::Archived => MRFEProjectStatus::Archived,
            ProjectStatus::Processing => MRFEProjectStatus::Processing,
            ProjectStatus::Withheld => MRFEProjectStatus::Withheld,
            ProjectStatus::Unknown => MRFEProjectStatus::Unknown,
        }
    }
}

impl From<MRFEProjectStatus> for ProjectStatus {
    fn from(value: MRFEProjectStatus) -> Self {
        match value {
            MRFEProjectStatus::Approved => ProjectStatus::Approved,
            MRFEProjectStatus::Rejected => ProjectStatus::Rejected,
            MRFEProjectStatus::Draft => ProjectStatus::Draft,
            MRFEProjectStatus::Unlisted => ProjectStatus::Unlisted,
            MRFEProjectStatus::Archived => ProjectStatus::Archived,
            MRFEProjectStatus::Processing => ProjectStatus::Processing,
            MRFEProjectStatus::Withheld => ProjectStatus::Withheld,
            MRFEProjectStatus::Unknown => ProjectStatus::Unknown,
        }
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MRFEUserRole {
    Developer,
    Moderator,
    Admin,
}

impl From<UserRole> for MRFEUserRole {
    fn from(value: UserRole) -> Self {
        match value {
            UserRole::Admin => Self::Admin,
            UserRole::Developer => Self::Developer,
            UserRole::Moderator => Self::Moderator,
        }
    }
}

impl From<MRFEUserRole> for UserRole {
    fn from(value: MRFEUserRole) -> Self {
        match value {
            MRFEUserRole::Admin => Self::Admin,
            MRFEUserRole::Developer => Self::Developer,
            MRFEUserRole::Moderator => Self::Moderator,
        }
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct MRFEUser {
    pub username: String,
    /// The user's display name
    pub name: Option<String>,
    /// The user's email, only visible to the user itself when authenticated
    pub email: Option<String>,
    /// A description of the user
    pub bio: Option<String>,
    /// Various data relating to the user's payouts status,
    /// only visible to the user itself when authenticated
    pub id: String,
    /// The user's GitHub ID
    pub github_id: Option<u32>,
    pub avatar_url: Option<String>,
    pub created: String,
    pub role: MRFEUserRole,
    /// Any badges applicable to this user.
    /// These are currently unused and not displayed, and as such are subject to change.
    ///
    /// [documentation](https://docs.modrinth.com/api-spec/#tag/user_model)
    pub badges: u32,
}

impl From<User> for MRFEUser {
    fn from(value: User) -> Self {
        Self {
            username: value.username,
            name: value.name,
            email: value.email,
            bio: value.bio,
            id: value.id,
            github_id: value.github_id,
            avatar_url: value.avatar_url,
            created: value.created.to_rfc3339(),
            role: value.role.into(),
            badges: value.badges,
        }
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct MRFETeamMember {
    /// The ID of the member's team
    pub team_id: String,
    pub user: MRFEUser,
    pub role: String,
    pub ordering: Option<u32>,
}

impl From<TeamMember> for MRFETeamMember {
    fn from(value: TeamMember) -> Self {
        Self {
            team_id: value.team_id,
            user: value.user.into(),
            role: value.role,
            ordering: value.ordering,
        }
    }
}
