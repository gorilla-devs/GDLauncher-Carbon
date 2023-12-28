//! Models related to projects
//!
//! [documentation](https://docs.modrinth.com/api-spec/#tag/project_model)

use std::path::PathBuf;

use carbon_macro::into_query_parameters;
use daedalus::minecraft::VersionManifest;

use crate::domain::{
    runtime_path::InstancePath,
    url::{deserialize_from_raw_json, serialize_as_raw_json},
};

use super::{search::ProjectID, *};

#[into_query_parameters]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ProjectVersionsFilters {
    pub project_id: ProjectID,
    #[serde(
        serialize_with = "serialize_as_raw_json",
        deserialize_with = "deserialize_from_raw_json",
        skip_serializing_if = "Option::is_none"
    )]
    pub game_versions: Option<Vec<String>>,
    #[serde(
        serialize_with = "serialize_as_raw_json",
        deserialize_with = "deserialize_from_raw_json",
        skip_serializing_if = "Option::is_none"
    )]
    pub loaders: Option<Vec<String>>,
    pub offset: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Project {
    /// The project's slug, used for vanity URLs.
    /// This can change at any time, so use the [`Project::id`] for long term storage.
    pub slug: String,
    pub title: String,
    /// A short description of the project
    pub description: String,
    pub categories: Vec<String>,
    pub client_side: ProjectSupportRange,
    pub server_side: ProjectSupportRange,
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
    pub donation_urls: Vec<DonationLink>,
    pub project_type: ProjectType,
    pub downloads: u32,
    pub icon_url: Option<String>,
    /// The RGB color of the project, automatically generated from the project icon
    pub color: Option<u32>,
    pub id: String,
    /// The ID of the team that has ownership of this project
    pub team: String,
    /// A link to the long description of the project (only present for old projects)
    #[deprecated = "Read from `body` instead"]
    pub body_url: Option<String>,
    pub moderator_message: Option<ModeratorMessage>,
    pub published: UtcDateTime,
    pub updated: UtcDateTime,
    /// The date the project's status was set to approved or unlisted
    pub approved: Option<UtcDateTime>,
    pub followers: u32,
    pub status: ProjectStatus,
    pub license: License,
    /// A list of the version IDs of the project.
    /// This will only ever be empty if the project is a draft.
    pub versions: Vec<String>,
    /// A list of all of the game versions supported by the project
    pub game_versions: Vec<String>,
    /// A list of all of the loaders supported by the project
    pub loaders: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    /// A list of images that have been uploaded to the project's gallery
    pub gallery: Vec<GalleryItem>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ModeratorMessage {
    pub message: String,
    /// The longer body of the message
    pub body: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct License {
    /// The SPDX license ID of a project
    pub id: String,
    /// The license's long name
    pub name: String,
    /// The URL to this license
    pub url: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct DonationLink {
    /// The donation platform's ID
    pub id: String,
    pub platform: String,
    /// A link to the donation platform and user
    pub url: String,
}

/// An image that have been uploaded to a project's gallery
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GalleryItem {
    pub url: String,
    pub featured: bool,
    pub title: Option<String>,
    pub description: Option<String>,
    pub created: UtcDateTime,
    /// The order of the gallery image.
    /// Gallery images are sorted by this field and then alphabetically by title.
    pub ordering: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectDependencies {
    pub projects: Vec<Project>,
    pub versions: Vec<version::Version>,
}

/// Fields to edit on the projects specified
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EditMultipleProjectsBody {
    /// Set all of the categories to the categories specified here
    pub categories: Vec<String>,
    /// Add all of the categories specified here
    pub add_categories: Vec<String>,
    /// Remove all of the categories specified here
    pub remove_categories: Vec<String>,
    /// Set all of the additional categories to the categories specified here
    pub additional_categories: Vec<String>,
    /// Add all of the additional categories specified here
    pub add_additional_categories: Vec<String>,
    /// Remove all of the additional categories specified here
    pub remove_additional_categories: Vec<String>,
    /// Set all of the donation links to the donation links specified here
    pub donation_urls: Vec<DonationLink>,
    /// Add all of the donation links specified here
    pub add_donation_urls: Vec<DonationLink>,
    /// Remove all of the donation links specified here
    pub remove_donation_urls: Vec<DonationLink>,
    /// A link to where to submit bugs or issues with the projects
    pub issues_url: Option<String>,
    /// A link to the source code of the projects
    pub source_url: Option<String>,
    /// A link to the projects' wiki page or other relevant information
    pub wiki_url: Option<String>,
    /// An optional invite link to the projects' discord
    pub discord_url: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProjectStatus {
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

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RequestedProjectStatus {
    Approved,
    Archived,
    Unlisted,
    Private,
    Draft,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProjectSupportRange {
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

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    /// WARNING: Can also be a plugin or data pack.
    /// You will have to read the loaders to get more specific information.
    Mod,
    Shader,
    Modpack,
    ResourcePack,
}

impl ProjectType {
    pub fn into_path(
        self,
        instance_path: &InstancePath,
        game_version: String,
        mc_manifest: &VersionManifest,
    ) -> PathBuf {
        match self {
            Self::ResourcePack => {
                // anything older than 1.6.1 is in the texturepacks folder
                let index_1_6_1 = mc_manifest
                    .versions
                    .iter()
                    .position(|v| v.id == "1.6.1")
                    .unwrap_or(0);

                let manifest_version = mc_manifest
                    .versions
                    .iter()
                    .position(|v| v.id == game_version)
                    .unwrap_or(0);

                // current game version needs to be below 1.6.1, so the index needs to be higher
                if manifest_version > index_1_6_1 {
                    instance_path.get_texturepacks_path()
                } else {
                    instance_path.get_resourcepacks_path()
                }
            }
            Self::Shader => instance_path.get_shaderpacks_path(),
            _ => instance_path.get_mods_path(),
        }
    }
}

/// File extensions for images
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageFileExt {
    /// [Portable Network Graphics](https://en.wikipedia.org/wiki/PNG)
    PNG,
    /// [Joint Photographic Experts Group](https://en.wikipedia.org/wiki/JPEG)
    JPG,
    /// [Joint Photographic Experts Group](https://en.wikipedia.org/wiki/JPEG)
    JPEG,
    /// [Bitmap](https://en.wikipedia.org/wiki/BMP_file_format)
    BMP,
    /// [Graphics Interchange Format](https://en.wikipedia.org/wiki/GIF)
    GIF,
    /// [Web Picture](https://en.wikipedia.org/wiki/WebP)
    WebP,
    /// [Scalable Vector Graphics](https://en.wikipedia.org/wiki/SVG)
    SVG,
    /// [Scalable Vector Graphics](https://en.wikipedia.org/wiki/SVG#Compression) (gZip compressed)
    SVGZ,
    /// [Silicon Graphics Image](https://en.wikipedia.org/wiki/Silicon_Graphics_Image)
    RGB,
}

impl std::fmt::Display for ImageFileExt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("{:?}", self).to_lowercase())
    }
}
