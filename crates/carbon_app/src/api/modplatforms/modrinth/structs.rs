use std::collections::HashMap;

use rspc::Type;
use serde::{Deserialize, Serialize};

use crate::domain::modplatforms::modrinth::{
    project::{
        DonationLink, GalleryItem, License, ModeratorMessage, Project, ProjectStatus,
        ProjectSupportRange, ProjectType,
    },
    search::ProjectSearchResult,
    tag::Category,
    version::{
        AdditionalFileType, Dependency, DependencyType, HashAlgorithm, Hashes,
        RequestedVersionStatus, Status, Version, VersionFile, VersionType,
    },
};

#[derive(Type, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FEModrinthProjectSupportRange {
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

impl From<ProjectSupportRange> for FEModrinthProjectSupportRange {
    fn from(value: ProjectSupportRange) -> Self {
        match value {
            ProjectSupportRange::Required => FEModrinthProjectSupportRange::Required,
            ProjectSupportRange::Unsupported => FEModrinthProjectSupportRange::Unsupported,
            ProjectSupportRange::Optional => FEModrinthProjectSupportRange::Optional,
            ProjectSupportRange::Unknown => FEModrinthProjectSupportRange::Unknown,
        }
    }
}

impl From<FEModrinthProjectSupportRange> for ProjectSupportRange {
    fn from(value: FEModrinthProjectSupportRange) -> Self {
        match value {
            FEModrinthProjectSupportRange::Required => ProjectSupportRange::Required,
            FEModrinthProjectSupportRange::Unsupported => ProjectSupportRange::Unsupported,
            FEModrinthProjectSupportRange::Optional => ProjectSupportRange::Optional,
            FEModrinthProjectSupportRange::Unknown => ProjectSupportRange::Unknown,
        }
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FEModrinthProjectType {
    /// WARNING: Can also be a plugin or data pack.
    /// You will have to read the loaders to get more specific information.
    Mod,
    Shader,
    Modpack,
    ResourcePack,
}

impl From<ProjectType> for FEModrinthProjectType {
    fn from(value: ProjectType) -> Self {
        match value {
            ProjectType::Mod => FEModrinthProjectType::Mod,
            ProjectType::Shader => FEModrinthProjectType::Shader,
            ProjectType::Modpack => FEModrinthProjectType::Modpack,
            ProjectType::ResourcePack => FEModrinthProjectType::ResourcePack,
        }
    }
}

impl From<FEModrinthProjectType> for ProjectType {
    fn from(value: FEModrinthProjectType) -> Self {
        match value {
            FEModrinthProjectType::Mod => ProjectType::Mod,
            FEModrinthProjectType::Shader => ProjectType::Shader,
            FEModrinthProjectType::Modpack => ProjectType::Modpack,
            FEModrinthProjectType::ResourcePack => ProjectType::ResourcePack,
        }
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct FEModrinthProjectSearchResult {
    /// The slug of a project used for vanity urls.
    pub slug: String,
    /// The title or name of the project
    pub title: String,
    /// A short description of the project
    pub description: String,
    /// A list of the categories that the project has
    pub categories: Option<Vec<String>>,
    /// The client side support of the project
    pub client_side: FEModrinthProjectSupportRange,
    /// The server side support of the project
    pub server_side: FEModrinthProjectSupportRange,
    /// The project type of the project
    pub project_type: FEModrinthProjectType,
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

impl From<ProjectSearchResult> for FEModrinthProjectSearchResult {
    fn from(value: ProjectSearchResult) -> Self {
        FEModrinthProjectSearchResult {
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

impl TryFrom<FEModrinthProjectSearchResult> for ProjectSearchResult {
    type Error = anyhow::Error;
    fn try_from(value: FEModrinthProjectSearchResult) -> Result<Self, Self::Error> {
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
pub struct FEModrinthCategory {
    /// An SVG icon for the category
    pub icon: String,
    pub name: String,
    /// The project type this category is applicable to
    pub project_type: FEModrinthProjectType,
    /// The header under which the category should go
    pub header: String,
}

impl From<Category> for FEModrinthCategory {
    fn from(value: Category) -> Self {
        FEModrinthCategory {
            icon: value.icon,
            name: value.name,
            project_type: value.project_type.into(),
            header: value.header,
        }
    }
}

impl From<FEModrinthCategory> for Category {
    fn from(value: FEModrinthCategory) -> Self {
        Category {
            icon: value.icon,
            name: value.name,
            project_type: value.project_type.into(),
            header: value.header,
        }
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct FEModrinthVersion {
    pub name: String,
    /// The version number.
    /// Ideally, this will follow semantic versioning.
    pub version_number: String,
    pub changelog: Option<String>,
    pub dependencies: Vec<FEModrinthDependency>,
    pub game_versions: Vec<String>,
    /// The release channel for this version
    pub version_type: FEModrinthVersionType,
    pub loaders: Vec<String>,
    pub featured: bool,
    pub status: Option<FEModrinthStatus>,
    pub requested_status: Option<FEModrinthRequestedVersionStatus>,
    pub id: String,
    /// The ID of the project this version is for
    pub project_id: String,
    /// The ID of the author who published this version
    pub author_id: String,
    pub date_published: String,
    pub downloads: u32,
    /// A list of files available for download
    pub files: Vec<FEModrinthVersionFile>,
}

impl From<Version> for FEModrinthVersion {
    fn from(value: Version) -> Self {
        FEModrinthVersion {
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

impl TryFrom<FEModrinthVersion> for Version {
    type Error = anyhow::Error;

    fn try_from(value: FEModrinthVersion) -> Result<Self, Self::Error> {
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
pub struct FEModrinthVersionFile {
    pub hashes: FEModrinthHashes,
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
    pub file_type: Option<FEModrinthAdditionalFileType>,
}

impl From<VersionFile> for FEModrinthVersionFile {
    fn from(value: VersionFile) -> Self {
        FEModrinthVersionFile {
            hashes: value.hashes.into(),
            url: value.url,
            filename: value.filename,
            primary: value.primary,
            size: value.size,
            file_type: value.file_type.map(Into::into),
        }
    }
}

impl From<FEModrinthVersionFile> for VersionFile {
    fn from(value: FEModrinthVersionFile) -> Self {
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
pub struct FEModrinthHashes {
    pub sha512: String,
    pub sha1: String,
    /// A map of other hashes that may have been provided
    #[serde(flatten)]
    pub others: HashMap<String, String>,
}

impl From<Hashes> for FEModrinthHashes {
    fn from(value: Hashes) -> Self {
        FEModrinthHashes {
            sha512: value.sha512,
            sha1: value.sha1,
            others: value.others,
        }
    }
}

impl From<FEModrinthHashes> for Hashes {
    fn from(value: FEModrinthHashes) -> Self {
        Hashes {
            sha512: value.sha512,
            sha1: value.sha1,
            others: value.others,
        }
    }
}

#[derive(Type, Serialize, Deserialize, Clone, Debug)]
pub struct FEModrinthDependency {
    pub version_id: Option<String>,
    pub project_id: Option<String>,
    pub file_name: Option<String>,
    pub dependency_type: FEModrinthDependencyType,
}

impl From<Dependency> for FEModrinthDependency {
    fn from(value: Dependency) -> Self {
        FEModrinthDependency {
            version_id: value.version_id,
            project_id: value.project_id,
            file_name: value.file_name,
            dependency_type: value.dependency_type.into(),
        }
    }
}

impl From<FEModrinthDependency> for Dependency {
    fn from(value: FEModrinthDependency) -> Self {
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
pub enum FEModrinthHashAlgorithm {
    SHA512,
    SHA1,
}

impl From<HashAlgorithm> for FEModrinthHashAlgorithm {
    fn from(value: HashAlgorithm) -> Self {
        match value {
            HashAlgorithm::SHA512 => FEModrinthHashAlgorithm::SHA512,
            HashAlgorithm::SHA1 => FEModrinthHashAlgorithm::SHA1,
        }
    }
}

impl From<FEModrinthHashAlgorithm> for HashAlgorithm {
    fn from(value: FEModrinthHashAlgorithm) -> Self {
        match value {
            FEModrinthHashAlgorithm::SHA512 => HashAlgorithm::SHA512,
            FEModrinthHashAlgorithm::SHA1 => HashAlgorithm::SHA1,
        }
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FEModrinthVersionType {
    Alpha,
    Beta,
    Release,
}

impl From<VersionType> for FEModrinthVersionType {
    fn from(value: VersionType) -> Self {
        match value {
            VersionType::Alpha => FEModrinthVersionType::Alpha,
            VersionType::Beta => FEModrinthVersionType::Beta,
            VersionType::Release => FEModrinthVersionType::Release,
        }
    }
}

impl From<FEModrinthVersionType> for VersionType {
    fn from(value: FEModrinthVersionType) -> Self {
        match value {
            FEModrinthVersionType::Alpha => VersionType::Alpha,
            FEModrinthVersionType::Beta => VersionType::Beta,
            FEModrinthVersionType::Release => VersionType::Release,
        }
    }
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FEModrinthDependencyType {
    Required,
    Optional,
    Incompatible,
    Embedded,
}

impl From<DependencyType> for FEModrinthDependencyType {
    fn from(value: DependencyType) -> Self {
        match value {
            DependencyType::Required => FEModrinthDependencyType::Required,
            DependencyType::Optional => FEModrinthDependencyType::Optional,
            DependencyType::Incompatible => FEModrinthDependencyType::Incompatible,
            DependencyType::Embedded => FEModrinthDependencyType::Embedded,
        }
    }
}

impl From<FEModrinthDependencyType> for DependencyType {
    fn from(value: FEModrinthDependencyType) -> Self {
        match value {
            FEModrinthDependencyType::Required => DependencyType::Required,
            FEModrinthDependencyType::Optional => DependencyType::Optional,
            FEModrinthDependencyType::Incompatible => DependencyType::Incompatible,
            FEModrinthDependencyType::Embedded => DependencyType::Embedded,
        }
    }
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FEModrinthStatus {
    Listed,
    Archived,
    Draft,
    Unlisted,
    Scheduled,
    Unknown,
}

impl From<Status> for FEModrinthStatus {
    fn from(value: Status) -> Self {
        match value {
            Status::Listed => FEModrinthStatus::Listed,
            Status::Archived => FEModrinthStatus::Archived,
            Status::Draft => FEModrinthStatus::Draft,
            Status::Unlisted => FEModrinthStatus::Unlisted,
            Status::Scheduled => FEModrinthStatus::Scheduled,
            Status::Unknown => FEModrinthStatus::Unknown,
        }
    }
}

impl From<FEModrinthStatus> for Status {
    fn from(value: FEModrinthStatus) -> Self {
        match value {
            FEModrinthStatus::Listed => Status::Listed,
            FEModrinthStatus::Archived => Status::Archived,
            FEModrinthStatus::Draft => Status::Draft,
            FEModrinthStatus::Unlisted => Status::Unlisted,
            FEModrinthStatus::Scheduled => Status::Scheduled,
            FEModrinthStatus::Unknown => Status::Unknown,
        }
    }
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FEModrinthRequestedVersionStatus {
    Listed,
    Archived,
    Draft,
    Unlisted,
}

impl From<RequestedVersionStatus> for FEModrinthRequestedVersionStatus {
    fn from(value: RequestedVersionStatus) -> Self {
        match value {
            RequestedVersionStatus::Listed => FEModrinthRequestedVersionStatus::Listed,
            RequestedVersionStatus::Archived => FEModrinthRequestedVersionStatus::Archived,
            RequestedVersionStatus::Draft => FEModrinthRequestedVersionStatus::Draft,
            RequestedVersionStatus::Unlisted => FEModrinthRequestedVersionStatus::Unlisted,
        }
    }
}

impl From<FEModrinthRequestedVersionStatus> for RequestedVersionStatus {
    fn from(value: FEModrinthRequestedVersionStatus) -> Self {
        match value {
            FEModrinthRequestedVersionStatus::Listed => RequestedVersionStatus::Listed,
            FEModrinthRequestedVersionStatus::Archived => RequestedVersionStatus::Archived,
            FEModrinthRequestedVersionStatus::Draft => RequestedVersionStatus::Draft,
            FEModrinthRequestedVersionStatus::Unlisted => RequestedVersionStatus::Unlisted,
        }
    }
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FEModrinthAdditionalFileType {
    RequiredResourcePack,
    OptionalResourcePack,
}

impl From<AdditionalFileType> for FEModrinthAdditionalFileType {
    fn from(value: AdditionalFileType) -> Self {
        match value {
            AdditionalFileType::RequiredResourcePack => {
                FEModrinthAdditionalFileType::RequiredResourcePack
            }
            AdditionalFileType::OptionalResourcePack => {
                FEModrinthAdditionalFileType::OptionalResourcePack
            }
        }
    }
}

impl From<FEModrinthAdditionalFileType> for AdditionalFileType {
    fn from(value: FEModrinthAdditionalFileType) -> Self {
        match value {
            FEModrinthAdditionalFileType::RequiredResourcePack => {
                AdditionalFileType::RequiredResourcePack
            }
            FEModrinthAdditionalFileType::OptionalResourcePack => {
                AdditionalFileType::OptionalResourcePack
            }
        }
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct FEModrinthProject {
    /// The project's slug, used for vanity URLs.
    /// This can change at any time, so use the [`Project::id`] for long term storage.
    pub slug: String,
    pub title: String,
    /// A short description of the project
    pub description: String,
    pub categories: Vec<String>,
    pub client_side: FEModrinthProjectSupportRange,
    pub server_side: FEModrinthProjectSupportRange,
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
    pub donation_urls: Vec<FEModrinthDonationLink>,
    pub project_type: FEModrinthProjectType,
    pub downloads: u32,
    pub icon_url: Option<String>,
    /// The RGB color of the project, automatically generated from the project icon
    pub color: Option<u32>,
    pub id: String,
    /// The ID of the team that has ownership of this project
    pub team: String,
    /// A link to the long description of the project (only present for old projects)
    pub moderator_message: Option<FEModrinthModeratorMessage>,
    pub published: String,
    pub updated: String,
    /// The date the project's status was set to approved or unlisted
    pub approved: Option<String>,
    pub followers: u32,
    pub status: FEModrinthProjectStatus,
    pub license: FEModrinthLicense,
    /// A list of the version IDs of the project.
    /// This will only ever be empty if the project is a draft.
    pub versions: Vec<String>,
    /// A list of all of the game versions supported by the project
    pub game_versions: Vec<String>,
    /// A list of all of the loaders supported by the project
    pub loaders: Vec<String>,
    /// A list of images that have been uploaded to the project's gallery
    pub gallery: Vec<FEModrinthGalleryItem>,
}

impl From<Project> for FEModrinthProject {
    fn from(value: Project) -> Self {
        FEModrinthProject {
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

impl TryFrom<FEModrinthProject> for Project {
    type Error = anyhow::Error;
    fn try_from(value: FEModrinthProject) -> Result<Self, Self::Error> {
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
pub struct FEModrinthModeratorMessage {
    pub message: String,
    /// The longer body of the message
    pub body: Option<String>,
}

impl From<ModeratorMessage> for FEModrinthModeratorMessage {
    fn from(value: ModeratorMessage) -> Self {
        FEModrinthModeratorMessage {
            message: value.message,
            body: value.body,
        }
    }
}

impl From<FEModrinthModeratorMessage> for ModeratorMessage {
    fn from(value: FEModrinthModeratorMessage) -> Self {
        ModeratorMessage {
            message: value.message,
            body: value.body,
        }
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct FEModrinthLicense {
    /// The SPDX license ID of a project
    pub id: String,
    /// The license's long name
    pub name: String,
    /// The URL to this license
    pub url: Option<String>,
}

impl From<License> for FEModrinthLicense {
    fn from(value: License) -> Self {
        FEModrinthLicense {
            id: value.id,
            name: value.name,
            url: value.url,
        }
    }
}

impl From<FEModrinthLicense> for License {
    fn from(value: FEModrinthLicense) -> Self {
        License {
            id: value.id,
            name: value.name,
            url: value.url,
        }
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct FEModrinthDonationLink {
    /// The donation platform's ID
    pub id: String,
    pub platform: String,
    /// A link to the donation platform and user
    pub url: String,
}

impl From<DonationLink> for FEModrinthDonationLink {
    fn from(value: DonationLink) -> Self {
        FEModrinthDonationLink {
            id: value.id,
            platform: value.platform,
            url: value.url,
        }
    }
}

impl From<FEModrinthDonationLink> for DonationLink {
    fn from(value: FEModrinthDonationLink) -> Self {
        DonationLink {
            id: value.id,
            platform: value.platform,
            url: value.url,
        }
    }
}

/// An image that have been uploaded to a project's gallery
#[derive(Type, Serialize, Deserialize, Debug, Clone)]
pub struct FEModrinthGalleryItem {
    pub url: String,
    pub featured: bool,
    pub title: Option<String>,
    pub description: Option<String>,
    pub created: String,
    /// The order of the gallery image.
    /// Gallery images are sorted by this field and then alphabetically by title.
    pub ordering: u32,
}

impl From<GalleryItem> for FEModrinthGalleryItem {
    fn from(value: GalleryItem) -> Self {
        FEModrinthGalleryItem {
            url: value.url,
            featured: value.featured,
            title: value.title,
            description: value.description,
            created: value.created.to_rfc3339(),
            ordering: value.ordering,
        }
    }
}

impl TryFrom<FEModrinthGalleryItem> for GalleryItem {
    type Error = anyhow::Error;
    fn try_from(value: FEModrinthGalleryItem) -> Result<Self, Self::Error> {
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
pub enum FEModrinthProjectStatus {
    Approved,
    /// A moderator's message should be available on the project struct
    Rejected,
    Draft,
    /// The project has been approved and is publicly accessible, but will not show up in search results
    Unlisted,
    Archived,
    /// The project has been submitted for approval and is being reviewed
    Processing,
    Unknown,
}

impl From<ProjectStatus> for FEModrinthProjectStatus {
    fn from(value: ProjectStatus) -> Self {
        match value {
            ProjectStatus::Approved => FEModrinthProjectStatus::Approved,
            ProjectStatus::Rejected => FEModrinthProjectStatus::Rejected,
            ProjectStatus::Draft => FEModrinthProjectStatus::Draft,
            ProjectStatus::Unlisted => FEModrinthProjectStatus::Unlisted,
            ProjectStatus::Archived => FEModrinthProjectStatus::Archived,
            ProjectStatus::Processing => FEModrinthProjectStatus::Processing,
            ProjectStatus::Unknown => FEModrinthProjectStatus::Unknown,
        }
    }
}

impl From<FEModrinthProjectStatus> for ProjectStatus {
    fn from(value: FEModrinthProjectStatus) -> Self {
        match value {
            FEModrinthProjectStatus::Approved => ProjectStatus::Approved,
            FEModrinthProjectStatus::Rejected => ProjectStatus::Rejected,
            FEModrinthProjectStatus::Draft => ProjectStatus::Draft,
            FEModrinthProjectStatus::Unlisted => ProjectStatus::Unlisted,
            FEModrinthProjectStatus::Archived => ProjectStatus::Archived,
            FEModrinthProjectStatus::Processing => ProjectStatus::Processing,
            FEModrinthProjectStatus::Unknown => ProjectStatus::Unknown,
        }
    }
}
