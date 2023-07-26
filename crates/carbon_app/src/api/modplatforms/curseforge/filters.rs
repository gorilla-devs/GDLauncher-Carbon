use rspc::Type;
use serde::{Deserialize, Serialize};

use crate::domain::modplatforms::curseforge::filters::{
    FilesParameters, FilesParametersBody, ModDescriptionParameters, ModFileChangelogParameters,
    ModFileParameters, ModFilesParameters, ModFilesParametersQuery, ModParameters,
    ModSearchParameters, ModSearchParametersQuery, ModSearchSortField, ModSearchSortOrder,
    ModsParameters, ModsParametersBody,
};

use super::structs::{CFFEClassId, CFFEModLoaderType};

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CFFEModSearchSortField {
    Featured,
    Popularity,
    LastUpdated,
    Name,
    Author,
    TotalDownloads,
    Category,
    GameVersion,
}

impl From<CFFEModSearchSortField> for ModSearchSortField {
    fn from(field: CFFEModSearchSortField) -> Self {
        match field {
            CFFEModSearchSortField::Featured => ModSearchSortField::Featured,
            CFFEModSearchSortField::Popularity => ModSearchSortField::Popularity,
            CFFEModSearchSortField::LastUpdated => ModSearchSortField::LastUpdated,
            CFFEModSearchSortField::Name => ModSearchSortField::Name,
            CFFEModSearchSortField::Author => ModSearchSortField::Author,
            CFFEModSearchSortField::TotalDownloads => ModSearchSortField::TotalDownloads,
            CFFEModSearchSortField::Category => ModSearchSortField::Category,
            CFFEModSearchSortField::GameVersion => ModSearchSortField::GameVersion,
        }
    }
}

impl From<ModSearchSortField> for CFFEModSearchSortField {
    fn from(field: ModSearchSortField) -> Self {
        match field {
            ModSearchSortField::Featured => CFFEModSearchSortField::Featured,
            ModSearchSortField::Popularity => CFFEModSearchSortField::Popularity,
            ModSearchSortField::LastUpdated => CFFEModSearchSortField::LastUpdated,
            ModSearchSortField::Name => CFFEModSearchSortField::Name,
            ModSearchSortField::Author => CFFEModSearchSortField::Author,
            ModSearchSortField::TotalDownloads => CFFEModSearchSortField::TotalDownloads,
            ModSearchSortField::Category => CFFEModSearchSortField::Category,
            ModSearchSortField::GameVersion => CFFEModSearchSortField::GameVersion,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CFFEModSearchSortOrder {
    Ascending,
    Descending,
}

impl From<CFFEModSearchSortOrder> for ModSearchSortOrder {
    fn from(order: CFFEModSearchSortOrder) -> Self {
        match order {
            CFFEModSearchSortOrder::Ascending => ModSearchSortOrder::Ascending,
            CFFEModSearchSortOrder::Descending => ModSearchSortOrder::Descending,
        }
    }
}

impl From<ModSearchSortOrder> for CFFEModSearchSortOrder {
    fn from(order: ModSearchSortOrder) -> Self {
        match order {
            ModSearchSortOrder::Ascending => CFFEModSearchSortOrder::Ascending,
            ModSearchSortOrder::Descending => CFFEModSearchSortOrder::Descending,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CFFEModSearchParameters {
    pub query: CFFEModSearchParametersQuery,
}

impl From<CFFEModSearchParameters> for ModSearchParameters {
    fn from(params: CFFEModSearchParameters) -> Self {
        Self {
            query: params.query.into(),
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CFFEModSearchParametersQuery {
    pub game_id: i32,
    pub search_filter: Option<String>,
    pub game_version: Option<String>,
    pub category_ids: Option<Vec<i32>>,
    pub sort_order: Option<CFFEModSearchSortOrder>,
    pub sort_field: Option<CFFEModSearchSortField>,
    pub class_id: Option<CFFEClassId>,
    pub mod_loader_types: Option<Vec<CFFEModLoaderType>>,
    pub game_version_type_id: Option<i32>,
    pub author_id: Option<i32>,
    pub slug: Option<String>,
    pub index: Option<i32>,
    pub page_size: Option<i32>,
}

impl From<CFFEModSearchParametersQuery> for ModSearchParametersQuery {
    fn from(params: CFFEModSearchParametersQuery) -> Self {
        let mod_loader_types = params
            .mod_loader_types
            .map(|types| types.into_iter().map(|t| t.into()).collect());

        Self {
            game_id: params.game_id,
            search_filter: params.search_filter,
            game_version: params.game_version,
            category_ids: params.category_ids,
            sort_order: params.sort_order.map(Into::into),
            sort_field: params.sort_field.map(Into::into),
            class_id: params.class_id.map(Into::into),
            mod_loader_types,
            game_version_type_id: params.game_version_type_id,
            author_id: params.author_id,
            slug: params.slug,
            index: params.index,
            page_size: params.page_size,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CFFEModFilesParameters {
    pub mod_id: i32,
    pub query: CFFEModFilesParametersQuery,
}

impl From<CFFEModFilesParameters> for ModFilesParameters {
    fn from(params: CFFEModFilesParameters) -> Self {
        Self {
            mod_id: params.mod_id,
            query: params.query.into(),
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CFFEModFilesParametersQuery {
    #[specta(optional)]
    pub game_version: Option<String>,
    #[specta(optional)]
    pub mod_loader_type: Option<CFFEModLoaderType>,
    #[specta(optional)]
    pub game_version_type_id: Option<i32>,
    #[specta(optional)]
    pub index: Option<i32>,
    #[specta(optional)]
    pub page_size: Option<i32>,
}

impl From<CFFEModFilesParametersQuery> for ModFilesParametersQuery {
    fn from(params: CFFEModFilesParametersQuery) -> Self {
        Self {
            game_version: params.game_version,
            mod_loader_type: params.mod_loader_type.map(Into::into),
            game_version_type_id: params.game_version_type_id,
            index: params.index,
            page_size: params.page_size,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CFFEModFileParameters {
    pub mod_id: i32,
    pub file_id: i32,
}

impl From<CFFEModFileParameters> for ModFileParameters {
    fn from(params: CFFEModFileParameters) -> Self {
        Self {
            mod_id: params.mod_id,
            file_id: params.file_id,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CFFEModDescriptionParameters {
    pub mod_id: i32,
}

impl From<CFFEModDescriptionParameters> for ModDescriptionParameters {
    fn from(params: CFFEModDescriptionParameters) -> Self {
        Self {
            mod_id: params.mod_id,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CFFEModParameters {
    pub mod_id: i32,
}

impl From<CFFEModParameters> for ModParameters {
    fn from(params: CFFEModParameters) -> Self {
        Self {
            mod_id: params.mod_id,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CFFEModsParameters {
    pub body: CFFEModsParametersBody,
}

impl From<CFFEModsParameters> for ModsParameters {
    fn from(params: CFFEModsParameters) -> Self {
        Self {
            body: params.body.into(),
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CFFEModsParametersBody {
    pub mod_ids: Vec<i32>,
}

impl From<CFFEModsParametersBody> for ModsParametersBody {
    fn from(params: CFFEModsParametersBody) -> Self {
        Self {
            mod_ids: params.mod_ids,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CFFEFilesParameters {
    pub body: CFFEFilesParametersBody,
}

impl From<CFFEFilesParameters> for FilesParameters {
    fn from(params: CFFEFilesParameters) -> Self {
        Self {
            body: params.body.into(),
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CFFEFilesParametersBody {
    pub file_ids: Vec<i32>,
}

impl From<CFFEFilesParametersBody> for FilesParametersBody {
    fn from(params: CFFEFilesParametersBody) -> Self {
        Self {
            file_ids: params.file_ids,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CFFEModFileChangelogParameters {
    pub mod_id: i32,
    pub file_id: i32,
}

impl From<CFFEModFileChangelogParameters> for ModFileChangelogParameters {
    fn from(params: CFFEModFileChangelogParameters) -> Self {
        Self {
            mod_id: params.mod_id,
            file_id: params.file_id,
        }
    }
}
