use rspc::Type;
use serde::{Deserialize, Serialize};

use crate::domain::modplatforms::curseforge::filters::{
    FilesParameters, FilesParametersBody, ModDescriptionParameters, ModFileChangelogParameters,
    ModFileParameters, ModFilesParameters, ModFilesParametersQuery, ModParameters,
    ModSearchParameters, ModSearchParametersQuery, ModSearchSortField, ModSearchSortOrder,
    ModsParameters, ModsParametersBody,
};

use super::structs::{FEClassId, FEModLoaderType};

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FEModSearchSortField {
    Featured,
    Popularity,
    LastUpdated,
    Name,
    Author,
    TotalDownloads,
    Category,
    GameVersion,
}

impl From<FEModSearchSortField> for ModSearchSortField {
    fn from(field: FEModSearchSortField) -> Self {
        match field {
            FEModSearchSortField::Featured => ModSearchSortField::Featured,
            FEModSearchSortField::Popularity => ModSearchSortField::Popularity,
            FEModSearchSortField::LastUpdated => ModSearchSortField::LastUpdated,
            FEModSearchSortField::Name => ModSearchSortField::Name,
            FEModSearchSortField::Author => ModSearchSortField::Author,
            FEModSearchSortField::TotalDownloads => ModSearchSortField::TotalDownloads,
            FEModSearchSortField::Category => ModSearchSortField::Category,
            FEModSearchSortField::GameVersion => ModSearchSortField::GameVersion,
        }
    }
}

impl From<ModSearchSortField> for FEModSearchSortField {
    fn from(field: ModSearchSortField) -> Self {
        match field {
            ModSearchSortField::Featured => FEModSearchSortField::Featured,
            ModSearchSortField::Popularity => FEModSearchSortField::Popularity,
            ModSearchSortField::LastUpdated => FEModSearchSortField::LastUpdated,
            ModSearchSortField::Name => FEModSearchSortField::Name,
            ModSearchSortField::Author => FEModSearchSortField::Author,
            ModSearchSortField::TotalDownloads => FEModSearchSortField::TotalDownloads,
            ModSearchSortField::Category => FEModSearchSortField::Category,
            ModSearchSortField::GameVersion => FEModSearchSortField::GameVersion,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FEModSearchSortOrder {
    Ascending,
    Descending,
}

impl From<FEModSearchSortOrder> for ModSearchSortOrder {
    fn from(order: FEModSearchSortOrder) -> Self {
        match order {
            FEModSearchSortOrder::Ascending => ModSearchSortOrder::Ascending,
            FEModSearchSortOrder::Descending => ModSearchSortOrder::Descending,
        }
    }
}

impl From<ModSearchSortOrder> for FEModSearchSortOrder {
    fn from(order: ModSearchSortOrder) -> Self {
        match order {
            ModSearchSortOrder::Ascending => FEModSearchSortOrder::Ascending,
            ModSearchSortOrder::Descending => FEModSearchSortOrder::Descending,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEModSearchParameters {
    pub query: FEUnifiedSearchParameters,
}

impl From<FEModSearchParameters> for ModSearchParameters {
    fn from(params: FEModSearchParameters) -> Self {
        Self {
            query: params.query.into(),
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEUnifiedSearchParameters {
    pub game_id: i32,
    pub search_filter: Option<String>,
    pub game_version: Option<String>,
    pub category_ids: Option<Vec<i32>>,
    pub sort_order: Option<FEModSearchSortOrder>,
    pub sort_field: Option<FEModSearchSortField>,
    pub class_id: Option<FEClassId>,
    pub mod_loader_types: Option<Vec<FEModLoaderType>>,
    pub game_version_type_id: Option<i32>,
    pub author_id: Option<i32>,
    pub slug: Option<String>,
    pub index: Option<i32>,
    pub page_size: Option<i32>,
}

impl From<FEUnifiedSearchParameters> for ModSearchParametersQuery {
    fn from(params: FEUnifiedSearchParameters) -> Self {
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
pub struct FEModFilesParameters {
    pub mod_id: i32,
    pub query: FEModFilesParametersQuery,
}

impl From<FEModFilesParameters> for ModFilesParameters {
    fn from(params: FEModFilesParameters) -> Self {
        Self {
            mod_id: params.mod_id,
            query: params.query.into(),
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEModFilesParametersQuery {
    #[specta(optional)]
    pub game_version: Option<String>,
    #[specta(optional)]
    pub mod_loader_type: Option<FEModLoaderType>,
    #[specta(optional)]
    pub game_version_type_id: Option<i32>,
    #[specta(optional)]
    pub index: Option<i32>,
    #[specta(optional)]
    pub page_size: Option<i32>,
}

impl From<FEModFilesParametersQuery> for ModFilesParametersQuery {
    fn from(params: FEModFilesParametersQuery) -> Self {
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
pub struct FEModFileParameters {
    pub mod_id: i32,
    pub file_id: i32,
}

impl From<FEModFileParameters> for ModFileParameters {
    fn from(params: FEModFileParameters) -> Self {
        Self {
            mod_id: params.mod_id,
            file_id: params.file_id,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEModDescriptionParameters {
    pub mod_id: i32,
}

impl From<FEModDescriptionParameters> for ModDescriptionParameters {
    fn from(params: FEModDescriptionParameters) -> Self {
        Self {
            mod_id: params.mod_id,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEModParameters {
    pub mod_id: i32,
}

impl From<FEModParameters> for ModParameters {
    fn from(params: FEModParameters) -> Self {
        Self {
            mod_id: params.mod_id,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEModsParameters {
    pub body: FEModsParametersBody,
}

impl From<FEModsParameters> for ModsParameters {
    fn from(params: FEModsParameters) -> Self {
        Self {
            body: params.body.into(),
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEModsParametersBody {
    pub mod_ids: Vec<i32>,
}

impl From<FEModsParametersBody> for ModsParametersBody {
    fn from(params: FEModsParametersBody) -> Self {
        Self {
            mod_ids: params.mod_ids,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEFilesParameters {
    pub body: FEFilesParametersBody,
}

impl From<FEFilesParameters> for FilesParameters {
    fn from(params: FEFilesParameters) -> Self {
        Self {
            body: params.body.into(),
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEFilesParametersBody {
    pub file_ids: Vec<i32>,
}

impl From<FEFilesParametersBody> for FilesParametersBody {
    fn from(params: FEFilesParametersBody) -> Self {
        Self {
            file_ids: params.file_ids,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEModFileChangelogParameters {
    pub mod_id: i32,
    pub file_id: i32,
}

impl From<FEModFileChangelogParameters> for ModFileChangelogParameters {
    fn from(params: FEModFileChangelogParameters) -> Self {
        Self {
            mod_id: params.mod_id,
            file_id: params.file_id,
        }
    }
}
