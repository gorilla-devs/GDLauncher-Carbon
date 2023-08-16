use carbon_macro::into_query_parameters;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::domain::url::serialize_as_raw_json;

use super::{ClassId, ModLoaderType};

#[derive(Debug, Serialize_repr, Deserialize_repr)]
#[serde(rename_all = "camelCase")]
#[repr(u8)]
pub enum ModSearchSortField {
    Featured = 1,
    Popularity = 2,
    LastUpdated = 3,
    Name = 4,
    Author = 5,
    TotalDownloads = 6,
    Category = 7,
    GameVersion = 8,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ModSearchSortOrder {
    #[serde(rename = "asc")]
    Ascending,
    #[serde(rename = "desc")]
    Descending,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModSearchParameters {
    pub query: ModSearchParametersQuery,
}

#[into_query_parameters]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModSearchParametersQuery {
    pub game_id: u32,
    pub search_filter: Option<String>,
    pub game_version: Option<String>,
    #[serde(serialize_with = "serialize_as_raw_json")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_ids: Option<Vec<u32>>,
    pub sort_order: Option<ModSearchSortOrder>,
    pub sort_field: Option<ModSearchSortField>,
    pub class_id: Option<ClassId>,
    #[serde(serialize_with = "serialize_as_raw_json")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mod_loader_types: Option<Vec<ModLoaderType>>,
    pub game_version_type_id: Option<u32>,
    pub author_id: Option<u32>,
    pub slug: Option<String>,
    pub index: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModFilesParameters {
    pub mod_id: u32,
    pub query: ModFilesParametersQuery,
}

#[into_query_parameters]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModFilesParametersQuery {
    pub game_version: Option<String>,
    pub mod_loader_type: Option<ModLoaderType>,
    pub game_version_type_id: Option<u32>,
    pub index: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModFileParameters {
    pub mod_id: u32,
    pub file_id: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModDescriptionParameters {
    pub mod_id: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModParameters {
    pub mod_id: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModsParameters {
    pub body: ModsParametersBody,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModsParametersBody {
    pub mod_ids: Vec<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilesParameters {
    pub body: FilesParametersBody,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilesParametersBody {
    pub file_ids: Vec<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModFileChangelogParameters {
    pub mod_id: u32,
    pub file_id: u32,
}
