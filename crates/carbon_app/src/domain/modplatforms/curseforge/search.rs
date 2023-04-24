use carbon_macro::into_query_parameters;
use serde::{Deserialize, Serialize};

use super::{ClassId, Mod, ModLoaderType};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModSearchSortField {
    Featured,
    Popularity,
    LastUpdated,
    Name,
    Author,
    TotalDownloads,
    Category,
    GameVersion,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ModSearchSortOrder {
    #[serde(rename = "asc")]
    Ascending,
    #[serde(rename = "desc")]
    Descending,
}

#[into_query_parameters]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModSearchParameters {
    pub game_id: i32,
    pub page: Option<i32>,
    pub search_filter: Option<String>,
    pub game_version: Option<String>,
    pub category_id: Option<i32>,
    pub sort_order: Option<ModSearchSortOrder>,
    pub sort_field: Option<ModSearchSortField>,
    pub class_id: Option<ClassId>,
    pub mod_loader_type: Option<ModLoaderType>,
    pub game_version_type_id: Option<i32>,
    pub author_id: Option<i32>,
    pub slug: Option<String>,
    pub index: Option<i32>,
    pub page_size: Option<i32>,
}
