use serde::{Deserialize, Serialize};
use specta::Type;

use super::curseforge;
use super::curseforge::structs::CFFEMod;
use super::modrinth;
use super::modrinth::structs::MRFEProjectSearchResult;
use super::FESearchAPI;

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEUnifiedSearchResult {
    pub title: String,
    pub description: String,
    pub image_url: Option<String>,
    pub id: String,
    pub last_updated: String,
}

impl From<CFFEMod> for FEUnifiedSearchResult {
    fn from(value: CFFEMod) -> Self {
        FEUnifiedSearchResult {
            title: value.name,
            description: value.summary,
            image_url: value.logo.as_ref().map(|logo| logo.thumbnail_url.clone()),
            id: value.id.to_string(),
            last_updated: value.date_modified.to_string(),
        }
    }
}

impl From<MRFEProjectSearchResult> for FEUnifiedSearchResult {
    fn from(value: MRFEProjectSearchResult) -> Self {
        FEUnifiedSearchResult {
            title: value.title,
            description: value.description,
            image_url: value.icon_url.map(|url| url),
            id: value.project_id,
            last_updated: value.date_modified,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEUnifiedPagination {
    pub index: u32,
    pub page_size: u32,
    pub result_count: u32,
    pub total_count: u32,
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEUnifiedSearchResponse {
    pub search_api: FESearchAPI,
    pub data: Vec<FEUnifiedSearchResult>,
    pub pagination: Option<FEUnifiedPagination>,
}

impl From<curseforge::responses::FEModSearchResponse> for FEUnifiedSearchResponse {
    fn from(value: curseforge::responses::FEModSearchResponse) -> Self {
        FEUnifiedSearchResponse {
            search_api: FESearchAPI::Curseforge,
            data: value
                .data
                .into_iter()
                .map(FEUnifiedSearchResult::from)
                .collect(),
            pagination: value.pagination.map(|pagination| FEUnifiedPagination {
                index: pagination.index as u32,
                page_size: pagination.page_size as u32,
                result_count: pagination.result_count as u32,
                total_count: pagination.total_count as u32,
            }),
        }
    }
}

impl From<modrinth::responses::MRFEProjectSearchResponse> for FEUnifiedSearchResponse {
    fn from(value: modrinth::responses::MRFEProjectSearchResponse) -> Self {
        let result_count = value.hits.len();
        FEUnifiedSearchResponse {
            search_api: FESearchAPI::Modrinth,
            data: value
                .hits
                .into_iter()
                .map(FEUnifiedSearchResult::from)
                .collect(),
            pagination: Some(FEUnifiedPagination {
                index: value.offset,
                page_size: value.limit,
                result_count: result_count as u32,
                total_count: value.total_hits,
            }),
        }
    }
}
