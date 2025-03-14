use serde::{Deserialize, Serialize};
use specta::Type;
use strum_macros::EnumIter;

use super::curseforge::structs::{CFFEClassId, CFFEMod};
use super::modrinth;
use super::modrinth::structs::{MRFEProjectSearchResult, MRFEProjectType};
use super::FESearchAPI;
use super::{curseforge, FEUnifiedSearchType};

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEUnifiedSearchResult {
    pub title: String,
    pub description: String,
    pub image_url: Option<String>,
    pub high_res_image_url: Option<String>,
    pub downloads_count: u32,
    pub id: String,
    pub last_updated: String,
    pub platform: String,
    pub r#type: FEUnifiedSearchType,
}

impl From<CFFEMod> for FEUnifiedSearchResult {
    fn from(value: CFFEMod) -> Self {
        FEUnifiedSearchResult {
            title: value.name,
            description: value.summary,
            image_url: value.logo.as_ref().map(|logo| logo.thumbnail_url.clone()),
            high_res_image_url: value.logo.as_ref().map(|logo| logo.url.clone()),
            id: value.id.to_string(),
            last_updated: value.date_modified.to_string(),
            downloads_count: value.download_count,
            platform: "curseforge".to_string(),
            r#type: value
                .class_id
                .map(|id| id.into())
                .unwrap_or(FEUnifiedSearchType::Unknown),
        }
    }
}

impl From<MRFEProjectSearchResult> for FEUnifiedSearchResult {
    fn from(value: MRFEProjectSearchResult) -> Self {
        FEUnifiedSearchResult {
            title: value.title,
            description: value.description,
            image_url: value.icon_url.as_ref().map(|url| url.clone()),
            high_res_image_url: value.icon_url.map(|url| url),
            id: value.project_id,
            last_updated: value.date_modified,
            downloads_count: value.downloads,
            platform: "modrinth".to_string(),
            r#type: value.project_type.into(),
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
    pub data: Vec<FEUnifiedSearchResult>,
    pub pagination: Option<FEUnifiedPagination>,
}

impl FEUnifiedSearchResponse {
    pub fn merge(
        cf_response: FEUnifiedSearchResponse,
        mr_response: FEUnifiedSearchResponse,
    ) -> Self {
        Self {
            data: cf_response
                .data
                .into_iter()
                .chain(mr_response.data.into_iter())
                .collect(),
            pagination: None,
        }
    }
}

impl From<curseforge::responses::FEModSearchResponse> for FEUnifiedSearchResponse {
    fn from(value: curseforge::responses::FEModSearchResponse) -> Self {
        FEUnifiedSearchResponse {
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
