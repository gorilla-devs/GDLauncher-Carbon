use super::FEUnifiedSearchType;
use super::responses::{FEUnifiedCategoryId, FEUnifiedModLoaderType, FEUnifiedPlatform};
use anyhow::anyhow;
use carbon_platforms::curseforge::filters::ModSearchParameters;
use carbon_platforms::modrinth::search::ProjectSearchParameters;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::fmt::Display;
use std::ops::{Deref, DerefMut};
use strum_macros::EnumIter;

#[derive(Type, Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Or<T>(pub Vec<T>);

impl<T> Deref for Or<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Or<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> IntoIterator for Or<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<T> FromIterator<T> for Or<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (size_lower, _) = iter.size_hint();
        let mut c = Vec::with_capacity(size_lower);
        for i in iter {
            c.push(i);
        }
        Or(c)
    }
}

#[derive(Type, Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct And<T>(pub Vec<T>);

impl<T> Deref for And<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for And<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> IntoIterator for And<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<T> FromIterator<T> for And<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (size_lower, _) = iter.size_hint();
        let mut c = Vec::with_capacity(size_lower);
        for i in iter {
            c.push(i);
        }
        And(c)
    }
}

impl<T> From<T> for And<T> {
    fn from(value: T) -> Self {
        And(vec![value])
    }
}

#[derive(Type, Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum FEUnifiedEnvironment {
    Server,
    Client,
}

#[derive(Type, Debug, Deserialize, Serialize, Clone, EnumIter)]
#[serde(rename_all = "camelCase")]
pub enum FEUnifiedModSortIndex {
    Relevance,   // for modrinth it's Relevance, for curseforge it's Popularity
    Downloads,   // for modrinth it's TotalDownloads, for curseforge it's Downloads
    LastUpdated, // for modrinth it's Updated, for curseforge it's LastUpdated
}

#[derive(Type, Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum FEUnifiedModSearchSortOrder {
    Ascending,
    Descending,
}

#[derive(Type, Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FEUnifiedSearchParameters {
    pub search_query: Option<String>,
    // Technically modrinth supports [AND[OR[category]]] but since curseforge doesn't, we will just support ANDs
    pub categories: Option<And<FEUnifiedCategoryId>>,
    pub game_versions: Option<Or<String>>,
    pub modloaders: Option<Or<FEUnifiedModLoaderType>>,
    pub project_type: Option<FEUnifiedSearchType>,
    pub sort_index: Option<FEUnifiedModSortIndex>,
    pub sort_order: Option<FEUnifiedModSearchSortOrder>,
    pub index: Option<u32>,
    pub page_size: Option<u32>,
    pub search_api: Option<FEUnifiedPlatform>,
    pub environment: Option<FEUnifiedEnvironment>,
}

impl From<FEUnifiedSearchParameters> for ProjectSearchParameters {
    fn from(value: FEUnifiedSearchParameters) -> Self {
        todo!()
    }
}

impl From<FEUnifiedSearchParameters> for ModSearchParameters {
    fn from(value: FEUnifiedSearchParameters) -> Self {
        todo!()
    }
}

#[derive(Type, Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type", content = "value")]
pub enum FEUnifiedProjectID {
    Curseforge(i32),
    Modrinth(String),
}

#[derive(Type, Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FEUnifiedProjectIDParameters {
    pub project_id: FEUnifiedProjectID,
}
