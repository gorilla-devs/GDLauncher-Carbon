use rspc::Type;
use serde::{Deserialize, Serialize};

use super::curseforge;
use super::curseforge::structs::FEMod;
use super::modrinth;
use super::modrinth::structs::FEModrinthProjectSearchResult;
use super::FESearchAPI;

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FEUnifiedSearchResult {
    Curseforge(FEMod),
    Modrinth(FEModrinthProjectSearchResult),
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
                .map(FEUnifiedSearchResult::Curseforge)
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

impl From<modrinth::responses::FEModrinthProjectSearchResponse> for FEUnifiedSearchResponse {
    fn from(value: modrinth::responses::FEModrinthProjectSearchResponse) -> Self {
        let result_count = value.hits.len();
        FEUnifiedSearchResponse {
            search_api: FESearchAPI::Modrinth,
            data: value
                .hits
                .into_iter()
                .map(FEUnifiedSearchResult::Modrinth)
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
