use crate::{
    api::{keys::modplatforms::CURSEFORGE_SEARCH, router::router},
    managers::App,
};
use carbon_macro::FromTo;
use rspc::{RouterBuilderLike, Type};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

pub(super) fn mount() -> impl RouterBuilderLike<App> {
    router! {
        query CURSEFORGE_SEARCH[app, filters: FEModSearchParameters] {
            let response = app.modplatforms_manager();
            response.curseforge_search(filters.into()).await?;

            Ok(())
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[derive(FromTo)]
#[to(crate::domain::modplatforms::curseforge::search::ModSearchParameters)]
pub struct FEModSearchParameters {
    pub game_id: i32,
    pub page: Option<i32>,
    pub search_filter: Option<String>,
    pub game_version: Option<String>,
    pub category_id: Option<i32>,
    pub sort_order: Option<FEModSearchSortOrder>,
    pub game_version_type_id: Option<i32>,
    pub author_id: Option<i32>,
    pub slug: Option<String>,
    pub index: Option<i32>,
    pub page_size: Option<i32>,
}

// #[derive(Type, Debug, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// #[derive(FromTo)]
// #[to(crate::domain::modplatforms::curseforge::search::ModSearchSortField)]
// pub enum FEModSearchSortField {
//     Featured,
//     Popularity,
//     LastUpdated,
//     Name,
//     Author,
//     TotalDownloads,
//     Category,
//     GameVersion,
// }

#[derive(Type, Debug, Serialize, Deserialize, FromTo)]
#[to(crate::domain::modplatforms::curseforge::search::ModSearchSortOrder)]
pub enum FEModSearchSortOrder {
    #[serde(rename = "asc")]
    Ascending,
    #[serde(rename = "desc")]
    Descending,
}

// #[derive(Type, Debug, Serialize_repr, Deserialize_repr, FromTo)]
// #[repr(u16)]
// #[to(crate::domain::modplatforms::curseforge::ClassId)]
// pub enum FEClassId {
//     Mods = 6,
//     Modpacks = 4471,
// }

// #[derive(Type, Debug, Serialize_repr, Deserialize_repr, FromTo)]
// #[repr(u8)]
// #[to(crate::domain::modplatforms::curseforge::ModLoaderType)]
// pub enum FEModLoaderType {
//     Any = 0,
//     Forge = 1,
//     Cauldron = 2,
//     LiteLoader = 3,
//     Fabric = 4,
//     Quilt = 5,
// }
