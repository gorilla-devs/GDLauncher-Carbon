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
pub enum FEProjectSupportRange {
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

impl From<ProjectSupportRange> for FEProjectSupportRange {
    fn from(value: ProjectSupportRange) -> Self {
        match value {
            ProjectSupportRange::Required => FEProjectSupportRange::Required,
            ProjectSupportRange::Unsupported => FEProjectSupportRange::Unsupported,
            ProjectSupportRange::Optional => FEProjectSupportRange::Optional,
            ProjectSupportRange::Unknown => FEProjectSupportRange::Unknown,
        }
    }
}

impl From<FEProjectSupportRange> for ProjectSupportRange {
    fn from(value: FEProjectSupportRange) -> Self {
        match value {
            FEProjectSupportRange::Required => ProjectSupportRange::Required,
            FEProjectSupportRange::Unsupported => ProjectSupportRange::Unsupported,
            FEProjectSupportRange::Optional => ProjectSupportRange::Optional,
            FEProjectSupportRange::Unknown => ProjectSupportRange::Unknown,
        }
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FEProjectType {
    /// WARNING: Can also be a plugin or data pack.
    /// You will have to read the loaders to get more specific information.
    Mod,
    Shader,
    Modpack,
    ResourcePack,
}

impl From<ProjectType> for FEProjectType {
    fn from(value: ProjectType) -> Self {
        match value {
            ProjectType::Mod => FEProjectType::Mod,
            ProjectType::Shader => FEProjectType::Shader,
            ProjectType::Modpack => FEProjectType::Modpack,
            ProjectType::ResourcePack => FEProjectType::ResourcePack,
        }
    }
}

impl From<FEProjectType> for ProjectType {
    fn from(value: FEProjectType) -> Self {
        match value {
            FEProjectType::Mod => ProjectType::Mod,
            FEProjectType::Shader => ProjectType::Shader,
            FEProjectType::Modpack => ProjectType::Modpack,
            FEProjectType::ResourcePack => ProjectType::ResourcePack,
        }
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct FEProjectSearchResult {
    /// The slug of a project used for vanity urls.
    pub slug: String,
    /// The title or name of the project
    pub title: String,
    /// A short description of the project
    pub description: String,
    /// A list of the categories that the project has
    pub categories: Option<Vec<String>>,
    /// The client side support of the project
    pub client_side: FEProjectSupportRange,
    /// The server side support of the project
    pub server_side: FEProjectSupportRange,
    /// The project type of the project
    pub project_type: FEProjectType,
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

impl From<ProjectSearchResult> for FEProjectSearchResult {
    fn from(value: ProjectSearchResult) -> Self {
        FEProjectSearchResult {
            slug: value.slug,
            title: value.title,
            description: value.description,
            categories: value.categories,
            client_side: value.client_side.into(),
            server_side: value.server_side.into(),
            project_type: value.project_type.into(),
            downloads: value.downloads,
            icon_url: value.icon_url.map(Into::into),
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

impl TryFrom<FEProjectSearchResult> for ProjectSearchResult {
    type Error = anyhow::Error;
    fn try_from(value: FEProjectSearchResult) -> Result<Self, Self::Error> {
        Ok(ProjectSearchResult {
            slug: value.slug,
            title: value.title,
            description: value.description,
            categories: value.categories,
            client_side: value.client_side.into(),
            server_side: value.server_side.into(),
            project_type: value.project_type.into(),
            downloads: value.downloads,
            icon_url: value
                .icon_url
                .map(|url| url.parse())
                .map_or(Ok(None), |url| url.map(Some))?,
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
pub struct FECategory {
    /// An SVG icon for the category
    pub icon: String,
    pub name: String,
    /// The project type this category is applicable to
    pub project_type: FEProjectType,
    /// The header under which the category should go
    pub header: String,
}

impl From<Category> for FECategory {
    fn from(value: Category) -> Self {
        FECategory {
            icon: value.icon,
            name: value.name,
            project_type: value.project_type.into(),
            header: value.header,
        }
    }
}

impl From<FECategory> for Category {
    fn from(value: FECategory) -> Self {
        Category {
            icon: value.icon,
            name: value.name,
            project_type: value.project_type.into(),
            header: value.header,
        }
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct FEVersion {
    pub name: String,
    /// The version number.
    /// Ideally, this will follow semantic versioning.
    pub version_number: String,
    pub changelog: Option<String>,
    pub dependencies: Vec<FEDependency>,
    pub game_versions: Vec<String>,
    /// The release channel for this version
    pub version_type: FEVersionType,
    pub loaders: Vec<String>,
    pub featured: bool,
    pub status: Option<FEStatus>,
    pub requested_status: Option<FERequestedVersionStatus>,
    pub id: String,
    /// The ID of the project this version is for
    pub project_id: String,
    /// The ID of the author who published this version
    pub author_id: String,
    pub date_published: String,
    pub downloads: u32,
    /// A list of files available for download
    pub files: Vec<FEVersionFile>,
}

impl From<Version> for FEVersion {
    fn from(value: Version) -> Self {
        FEVersion {
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

impl TryFrom<FEVersion> for Version {
    type Error = anyhow::Error;

    fn try_from(value: FEVersion) -> Result<Self, Self::Error> {
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
pub struct FEVersionFile {
    pub hashes: FEHashes,
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
    pub file_type: Option<FEAdditionalFileType>,
}

impl From<VersionFile> for FEVersionFile {
    fn from(value: VersionFile) -> Self {
        FEVersionFile {
            hashes: value.hashes.into(),
            url: value.url.into(),
            filename: value.filename,
            primary: value.primary,
            size: value.size,
            file_type: value.file_type.map(Into::into),
        }
    }
}

impl TryFrom<FEVersionFile> for VersionFile {
    type Error = anyhow::Error;

    fn try_from(value: FEVersionFile) -> Result<Self, Self::Error> {
        Ok(VersionFile {
            hashes: value.hashes.into(),
            url: value.url.parse()?,
            filename: value.filename,
            primary: value.primary,
            size: value.size,
            file_type: value.file_type.map(Into::into),
        })
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct FEHashes {
    pub sha512: String,
    pub sha1: String,
    /// A map of other hashes that may have been provided
    #[serde(flatten)]
    pub others: HashMap<String, String>,
}

impl From<Hashes> for FEHashes {
    fn from(value: Hashes) -> Self {
        FEHashes {
            sha512: value.sha512,
            sha1: value.sha1,
            others: value.others,
        }
    }
}

impl From<FEHashes> for Hashes {
    fn from(value: FEHashes) -> Self {
        Hashes {
            sha512: value.sha512,
            sha1: value.sha1,
            others: value.others,
        }
    }
}

#[derive(Type, Serialize, Deserialize, Clone, Debug)]
pub struct FEDependency {
    pub version_id: Option<String>,
    pub project_id: Option<String>,
    pub file_name: Option<String>,
    pub dependency_type: FEDependencyType,
}

impl From<Dependency> for FEDependency {
    fn from(value: Dependency) -> Self {
        FEDependency {
            version_id: value.version_id,
            project_id: value.project_id,
            file_name: value.file_name,
            dependency_type: value.dependency_type.into(),
        }
    }
}

impl From<FEDependency> for Dependency {
    fn from(value: FEDependency) -> Self {
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
pub enum FEHashAlgorithm {
    SHA512,
    SHA1,
}

impl From<HashAlgorithm> for FEHashAlgorithm {
    fn from(value: HashAlgorithm) -> Self {
        match value {
            HashAlgorithm::SHA512 => FEHashAlgorithm::SHA512,
            HashAlgorithm::SHA1 => FEHashAlgorithm::SHA1,
        }
    }
}

impl From<FEHashAlgorithm> for HashAlgorithm {
    fn from(value: FEHashAlgorithm) -> Self {
        match value {
            FEHashAlgorithm::SHA512 => HashAlgorithm::SHA512,
            FEHashAlgorithm::SHA1 => HashAlgorithm::SHA1,
        }
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FEVersionType {
    Alpha,
    Beta,
    Release,
}

impl From<VersionType> for FEVersionType {
    fn from(value: VersionType) -> Self {
        match value {
            VersionType::Alpha => FEVersionType::Alpha,
            VersionType::Beta => FEVersionType::Beta,
            VersionType::Release => FEVersionType::Release,
        }
    }
}

impl From<FEVersionType> for VersionType {
    fn from(value: FEVersionType) -> Self {
        match value {
            FEVersionType::Alpha => VersionType::Alpha,
            FEVersionType::Beta => VersionType::Beta,
            FEVersionType::Release => VersionType::Release,
        }
    }
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FEDependencyType {
    Required,
    Optional,
    Incompatible,
    Embedded,
}

impl From<DependencyType> for FEDependencyType {
    fn from(value: DependencyType) -> Self {
        match value {
            DependencyType::Required => FEDependencyType::Required,
            DependencyType::Optional => FEDependencyType::Optional,
            DependencyType::Incompatible => FEDependencyType::Incompatible,
            DependencyType::Embedded => FEDependencyType::Embedded,
        }
    }
}

impl From<FEDependencyType> for DependencyType {
    fn from(value: FEDependencyType) -> Self {
        match value {
            FEDependencyType::Required => DependencyType::Required,
            FEDependencyType::Optional => DependencyType::Optional,
            FEDependencyType::Incompatible => DependencyType::Incompatible,
            FEDependencyType::Embedded => DependencyType::Embedded,
        }
    }
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FEStatus {
    Listed,
    Archived,
    Draft,
    Unlisted,
    Scheduled,
    Unknown,
}

impl From<Status> for FEStatus {
    fn from(value: Status) -> Self {
        match value {
            Status::Listed => FEStatus::Listed,
            Status::Archived => FEStatus::Archived,
            Status::Draft => FEStatus::Draft,
            Status::Unlisted => FEStatus::Unlisted,
            Status::Scheduled => FEStatus::Scheduled,
            Status::Unknown => FEStatus::Unknown,
        }
    }
}

impl From<FEStatus> for Status {
    fn from(value: FEStatus) -> Self {
        match value {
            FEStatus::Listed => Status::Listed,
            FEStatus::Archived => Status::Archived,
            FEStatus::Draft => Status::Draft,
            FEStatus::Unlisted => Status::Unlisted,
            FEStatus::Scheduled => Status::Scheduled,
            FEStatus::Unknown => Status::Unknown,
        }
    }
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FERequestedVersionStatus {
    Listed,
    Archived,
    Draft,
    Unlisted,
}

impl From<RequestedVersionStatus> for FERequestedVersionStatus {
    fn from(value: RequestedVersionStatus) -> Self {
        match value {
            RequestedVersionStatus::Listed => FERequestedVersionStatus::Listed,
            RequestedVersionStatus::Archived => FERequestedVersionStatus::Archived,
            RequestedVersionStatus::Draft => FERequestedVersionStatus::Draft,
            RequestedVersionStatus::Unlisted => FERequestedVersionStatus::Unlisted,
        }
    }
}

impl From<FERequestedVersionStatus> for RequestedVersionStatus {
    fn from(value: FERequestedVersionStatus) -> Self {
        match value {
            FERequestedVersionStatus::Listed => RequestedVersionStatus::Listed,
            FERequestedVersionStatus::Archived => RequestedVersionStatus::Archived,
            FERequestedVersionStatus::Draft => RequestedVersionStatus::Draft,
            FERequestedVersionStatus::Unlisted => RequestedVersionStatus::Unlisted,
        }
    }
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FEAdditionalFileType {
    RequiredResourcePack,
    OptionalResourcePack,
}

impl From<AdditionalFileType> for FEAdditionalFileType {
    fn from(value: AdditionalFileType) -> Self {
        match value {
            AdditionalFileType::RequiredResourcePack => FEAdditionalFileType::RequiredResourcePack,
            AdditionalFileType::OptionalResourcePack => FEAdditionalFileType::OptionalResourcePack,
        }
    }
}

impl From<FEAdditionalFileType> for AdditionalFileType {
    fn from(value: FEAdditionalFileType) -> Self {
        match value {
            FEAdditionalFileType::RequiredResourcePack => AdditionalFileType::RequiredResourcePack,
            FEAdditionalFileType::OptionalResourcePack => AdditionalFileType::OptionalResourcePack,
        }
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct FEProject {
    /// The project's slug, used for vanity URLs.
    /// This can change at any time, so use the [`Project::id`] for long term storage.
    pub slug: String,
    pub title: String,
    /// A short description of the project
    pub description: String,
    pub categories: Vec<String>,
    pub client_side: FEProjectSupportRange,
    pub server_side: FEProjectSupportRange,
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
    pub donation_urls: Vec<FEDonationLink>,
    pub project_type: FEProjectType,
    pub downloads: u32,
    pub icon_url: Option<String>,
    /// The RGB color of the project, automatically generated from the project icon
    pub color: Option<u32>,
    pub id: String,
    /// The ID of the team that has ownership of this project
    pub team: String,
    /// A link to the long description of the project (only present for old projects)
    pub moderator_message: Option<FEModeratorMessage>,
    pub published: String,
    pub updated: String,
    /// The date the project's status was set to approved or unlisted
    pub approved: Option<String>,
    pub followers: u32,
    pub status: FEProjectStatus,
    pub license: FELicense,
    /// A list of the version IDs of the project.
    /// This will only ever be empty if the project is a draft.
    pub versions: Vec<String>,
    /// A list of all of the game versions supported by the project
    pub game_versions: Vec<String>,
    /// A list of all of the loaders supported by the project
    pub loaders: Vec<String>,
    /// A list of images that have been uploaded to the project's gallery
    pub gallery: Vec<FEGalleryItem>,
}

impl From<Project> for FEProject {
    fn from(value: Project) -> Self {
        FEProject {
            slug: value.slug,
            title: value.title,
            description: value.description,
            categories: value.categories,
            client_side: value.client_side.into(),
            server_side: value.server_side.into(),
            body: value.body,
            additional_categories: value.additional_categories,
            issues_url: value.issues_url.map(Into::into),
            source_url: value.source_url.map(Into::into),
            wiki_url: value.wiki_url.map(Into::into),
            discord_url: value.discord_url.map(Into::into),
            donation_urls: value.donation_urls.into_iter().map(Into::into).collect(),
            project_type: value.project_type.into(),
            downloads: value.downloads,
            icon_url: value.icon_url.map(Into::into),
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

impl TryFrom<FEProject> for Project {
    type Error = anyhow::Error;
    fn try_from(value: FEProject) -> Result<Self, Self::Error> {
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
            issues_url: value
                .issues_url
                .map(|url| url.parse())
                .map_or(Ok(None), |url| url.map(Some))?,
            source_url: value
                .source_url
                .map(|url| url.parse())
                .map_or(Ok(None), |url| url.map(Some))?,
            wiki_url: value
                .wiki_url
                .map(|url| url.parse())
                .map_or(Ok(None), |url| url.map(Some))?,
            discord_url: value
                .discord_url
                .map(|url| url.parse())
                .map_or(Ok(None), |url| url.map(Some))?,
            donation_urls: value
                .donation_urls
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<Vec<_>, _>>()?,
            project_type: value.project_type.into(),
            downloads: value.downloads,
            icon_url: value
                .icon_url
                .map(|url| url.parse())
                .map_or(Ok(None), |url| url.map(Some))?,
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
pub struct FEModeratorMessage {
    pub message: String,
    /// The longer body of the message
    pub body: Option<String>,
}

impl From<ModeratorMessage> for FEModeratorMessage {
    fn from(value: ModeratorMessage) -> Self {
        FEModeratorMessage {
            message: value.message,
            body: value.body,
        }
    }
}

impl From<FEModeratorMessage> for ModeratorMessage {
    fn from(value: FEModeratorMessage) -> Self {
        ModeratorMessage {
            message: value.message,
            body: value.body,
        }
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct FELicense {
    /// The SPDX license ID of a project
    pub id: String,
    /// The license's long name
    pub name: String,
    /// The URL to this license
    pub url: Option<String>,
}

impl From<License> for FELicense {
    fn from(value: License) -> Self {
        FELicense {
            id: value.id,
            name: value.name,
            url: value.url.map(Into::into),
        }
    }
}

impl TryFrom<FELicense> for License {
    type Error = anyhow::Error;
    fn try_from(value: FELicense) -> Result<Self, Self::Error> {
        Ok(License {
            id: value.id,
            name: value.name,
            url: value
                .url
                .map(|url| url.parse())
                .map_or(Ok(None), |url| url.map(Some))?,
        })
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct FEDonationLink {
    /// The donation platform's ID
    pub id: String,
    pub platform: String,
    /// A link to the donation platform and user
    pub url: String,
}

impl From<DonationLink> for FEDonationLink {
    fn from(value: DonationLink) -> Self {
        FEDonationLink {
            id: value.id,
            platform: value.platform,
            url: value.url.to_string(),
        }
    }
}

impl TryFrom<FEDonationLink> for DonationLink {
    type Error = anyhow::Error;
    fn try_from(value: FEDonationLink) -> Result<Self, Self::Error> {
        Ok(DonationLink {
            id: value.id,
            platform: value.platform,
            url: value.url.parse()?,
        })
    }
}

/// An image that have been uploaded to a project's gallery
#[derive(Type, Serialize, Deserialize, Debug, Clone)]
pub struct FEGalleryItem {
    pub url: String,
    pub featured: bool,
    pub title: Option<String>,
    pub description: Option<String>,
    pub created: String,
    /// The order of the gallery image.
    /// Gallery images are sorted by this field and then alphabetically by title.
    pub ordering: u32,
}

impl From<GalleryItem> for FEGalleryItem {
    fn from(value: GalleryItem) -> Self {
        FEGalleryItem {
            url: value.url.to_string(),
            featured: value.featured,
            title: value.title,
            description: value.description,
            created: value.created.to_rfc3339(),
            ordering: value.ordering,
        }
    }
}

impl TryFrom<FEGalleryItem> for GalleryItem {
    type Error = anyhow::Error;
    fn try_from(value: FEGalleryItem) -> Result<Self, Self::Error> {
        Ok(GalleryItem {
            url: value.url.parse()?,
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
pub enum FEProjectStatus {
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

impl From<ProjectStatus> for FEProjectStatus {
    fn from(value: ProjectStatus) -> Self {
        match value {
            ProjectStatus::Approved => FEProjectStatus::Approved,
            ProjectStatus::Rejected => FEProjectStatus::Rejected,
            ProjectStatus::Draft => FEProjectStatus::Draft,
            ProjectStatus::Unlisted => FEProjectStatus::Unlisted,
            ProjectStatus::Archived => FEProjectStatus::Archived,
            ProjectStatus::Processing => FEProjectStatus::Processing,
            ProjectStatus::Unknown => FEProjectStatus::Unknown,
        }
    }
}

impl From<FEProjectStatus> for ProjectStatus {
    fn from(value: FEProjectStatus) -> Self {
        match value {
            FEProjectStatus::Approved => ProjectStatus::Approved,
            FEProjectStatus::Rejected => ProjectStatus::Rejected,
            FEProjectStatus::Draft => ProjectStatus::Draft,
            FEProjectStatus::Unlisted => ProjectStatus::Unlisted,
            FEProjectStatus::Archived => ProjectStatus::Archived,
            FEProjectStatus::Processing => ProjectStatus::Processing,
            FEProjectStatus::Unknown => ProjectStatus::Unknown,
        }
    }
}
