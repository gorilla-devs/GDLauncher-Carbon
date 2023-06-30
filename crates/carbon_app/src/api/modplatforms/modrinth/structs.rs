use rspc::Type;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::domain::modplatforms::modrinth::{
    project::{ProjectSupportRange, ProjectType},
    search::ProjectSearchResult,
    UtcDateTime,
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
            icon_url: value.icon_url.map(|url| url.to_string()),
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
