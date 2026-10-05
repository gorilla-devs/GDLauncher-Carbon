use super::FEUnifiedSearchType;
use super::responses::{FEUnifiedCategoryId, FEUnifiedModLoaderType, FEUnifiedPlatform};
use anyhow::anyhow;
use carbon_platforms::curseforge::filters::{
    ModSearchParameters, ModSearchParametersQuery, ModSearchSortField, ModSearchSortOrder,
};
use carbon_platforms::modrinth::search::{
    ProjectSearchParameters, SearchFacet, SearchFacetAnd, SearchFacetOr, SearchIndex,
};
use carbon_platforms::modrinth::tag::LoaderType;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::fmt::Display;
use std::ops::{Deref, DerefMut};
use strum_macros::EnumIter;

#[derive(Type, Debug, Deserialize, Serialize, Clone)]
#[serde(transparent)]
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
#[serde(transparent)]
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

impl From<FEUnifiedModSortIndex> for SearchIndex {
    fn from(value: FEUnifiedModSortIndex) -> Self {
        match value {
            FEUnifiedModSortIndex::Relevance => SearchIndex::Relevance,
            FEUnifiedModSortIndex::Downloads => SearchIndex::Downloads,
            FEUnifiedModSortIndex::LastUpdated => SearchIndex::Updated,
        }
    }
}

impl From<FEUnifiedModSortIndex> for ModSearchSortField {
    fn from(value: FEUnifiedModSortIndex) -> Self {
        match value {
            FEUnifiedModSortIndex::Relevance => ModSearchSortField::Popularity,
            FEUnifiedModSortIndex::Downloads => ModSearchSortField::TotalDownloads,
            FEUnifiedModSortIndex::LastUpdated => ModSearchSortField::LastUpdated,
        }
    }
}
#[derive(Type, Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum FEUnifiedModSearchSortOrder {
    Ascending,
    Descending,
}

impl From<FEUnifiedModSearchSortOrder> for ModSearchSortOrder {
    fn from(value: FEUnifiedModSearchSortOrder) -> Self {
        match value {
            FEUnifiedModSearchSortOrder::Ascending => ModSearchSortOrder::Ascending,
            FEUnifiedModSearchSortOrder::Descending => ModSearchSortOrder::Descending,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum FECurseforgeSearchSortField {
    Featured,
    Popularity,
    TotalDownloads,
    LastUpdated,
    Name,
    Author,
    Category,
    GameVersion,
}

impl From<FECurseforgeSearchSortField> for ModSearchSortField {
    fn from(value: FECurseforgeSearchSortField) -> Self {
        match value {
            FECurseforgeSearchSortField::Featured => ModSearchSortField::Featured,
            FECurseforgeSearchSortField::Popularity => ModSearchSortField::Popularity,
            FECurseforgeSearchSortField::TotalDownloads => ModSearchSortField::TotalDownloads,
            FECurseforgeSearchSortField::LastUpdated => ModSearchSortField::LastUpdated,
            FECurseforgeSearchSortField::Name => ModSearchSortField::Name,
            FECurseforgeSearchSortField::Author => ModSearchSortField::Author,
            FECurseforgeSearchSortField::Category => ModSearchSortField::Category,
            FECurseforgeSearchSortField::GameVersion => ModSearchSortField::GameVersion,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum FEModrinthSearchIndex {
    Relevance,
    Downloads,
    Follows,
    Newest,
    Updated,
}

impl From<FEModrinthSearchIndex> for SearchIndex {
    fn from(value: FEModrinthSearchIndex) -> Self {
        match value {
            FEModrinthSearchIndex::Relevance => SearchIndex::Relevance,
            FEModrinthSearchIndex::Downloads => SearchIndex::Downloads,
            FEModrinthSearchIndex::Follows => SearchIndex::Follows,
            FEModrinthSearchIndex::Newest => SearchIndex::Newest,
            FEModrinthSearchIndex::Updated => SearchIndex::Updated,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "platform", content = "filters")]
pub enum FEPlatformFilters {
    Curseforge {
        sort_field: Option<FECurseforgeSearchSortField>,
        sort_order: Option<FEUnifiedModSearchSortOrder>,
    },
    Modrinth {
        sort_index: Option<FEModrinthSearchIndex>,
    },
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
    pub platform_filters: Option<FEPlatformFilters>,
    pub index: Option<u32>,
    pub page_size: Option<u32>,
    pub search_api: Option<FEUnifiedPlatform>,
    pub environment: Option<FEUnifiedEnvironment>,
}

impl From<FEUnifiedSearchParameters> for ProjectSearchParameters {
    fn from(value: FEUnifiedSearchParameters) -> Self {
        let mut facets = SearchFacetAnd::new();
        if let Some(categories) = value.categories {
            for cat_or in categories {
                match cat_or {
                    FEUnifiedCategoryId::Curseforge(_) => {}
                    FEUnifiedCategoryId::Modrinth(id) => {
                        facets.push(SearchFacetOr::new(vec![SearchFacet::Category(id)]));
                    }
                }
            }
        }
        if let Some(versions) = value.game_versions {
            let versions_or = versions.into_iter().map(SearchFacet::Version).collect();
            facets.push(versions_or);
        }
        if let Some(modloaders) = value.modloaders {
            let modloaders_or = modloaders
                .into_iter()
                .filter_map(|loader| TryInto::<LoaderType>::try_into(loader).ok())
                .map(|modloader| SearchFacet::Category(modloader.to_string()))
                .collect();
            facets.push(modloaders_or);
        }
        if let Some(project_type) = value.project_type {
            facets.push(SearchFacetOr::new(vec![SearchFacet::ProjectType(
                project_type.to_string(),
            )]));
        }

        // Extract Modrinth-specific sorting from platform filters, default to Downloads
        let sort_index = match &value.platform_filters {
            Some(FEPlatformFilters::Modrinth { sort_index }) => {
                sort_index.as_ref().map(|idx| idx.clone().into())
            }
            _ => Some(SearchIndex::Downloads), // Default to Downloads when no platform filters
        };

        let filters = value.environment.map(|env| match env {
            FEUnifiedEnvironment::Client => r#"client_side != "unsupported""#.to_string(),
            FEUnifiedEnvironment::Server => r#"server_side != "unsupported""#.to_string(),
        });

        ProjectSearchParameters {
            query: value.search_query,
            facets: if facets.is_empty() {
                None
            } else {
                Some(facets)
            },
            index: sort_index,
            offset: value.index,
            limit: value.page_size,
            filters,
        }
    }
}

impl TryFrom<FEUnifiedSearchParameters> for ModSearchParameters {
    type Error = anyhow::Error;
    fn try_from(value: FEUnifiedSearchParameters) -> anyhow::Result<Self> {
        let mut versions: Vec<String> = value
            .game_versions
            .clone()
            .unwrap_or(Or(Vec::new()))
            .into_iter()
            .filter(|v| !v.trim().is_empty())
            .collect();
        versions.sort();
        versions.dedup();
        anyhow::ensure!(
            versions.len() <= 1,
            "CurseForge supports one Minecraft version. Select one version or search Modrinth only."
        );
        // Extract Curseforge-specific sorting from platform filters, default to TotalDownloads DESC
        let (sort_field, sort_order) = match &value.platform_filters {
            Some(FEPlatformFilters::Curseforge {
                sort_field,
                sort_order,
            }) => (
                sort_field.as_ref().map(|f| f.clone().into()),
                sort_order.as_ref().map(|o| o.clone().into()),
            ),
            _ => (
                Some(ModSearchSortField::TotalDownloads),
                Some(ModSearchSortOrder::Descending),
            ), // Default to TotalDownloads DESC
        };

        Ok(ModSearchParameters {
            query: ModSearchParametersQuery {
                game_id: 432,
                search_filter: value.search_query,
                game_version: versions.into_iter().next(),
                category_ids: value.categories.map(|cat_groups| {
                    cat_groups
                        .into_iter()
                        .filter_map(|cat| {
                            // Curseforge does't support ORs of categories, take only the first of each
                            // group
                            match cat {
                                FEUnifiedCategoryId::Curseforge(id) => Some(id),
                                FEUnifiedCategoryId::Modrinth(_) => None,
                            }
                        })
                        .collect()
                }),
                sort_order,
                sort_field,
                class_id: value.project_type.map(Into::into),
                mod_loader_types: value.modloaders.map(|loaders| {
                    loaders
                        .into_iter()
                        .filter_map(|loader| loader.try_into().ok())
                        .collect()
                }),
                author_id: None,
                game_version_type_id: None,
                slug: None,
                index: value.index.map(|index| index as i32),
                page_size: value.page_size.map(|page_size| page_size as i32),
            },
        })
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

/// Batch request for fetching multiple projects by ID or slug
#[derive(Type, Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FEUnifiedBatchRequest {
    /// CurseForge numeric IDs (CF only)
    pub curseforge_ids: Vec<i32>,
    /// Slugs to try on BOTH platforms
    pub slugs: Vec<String>,
    /// Modrinth-specific IDs (from modrinth:// protocol or URL)
    pub modrinth_only_ids: Vec<String>,
    /// Explicit Modrinth version IDs, never sent to the project endpoint.
    #[serde(default)]
    pub modrinth_version_ids: Vec<String>,
    /// CurseForge-specific slugs (from CF URLs)
    pub curseforge_only_slugs: Vec<String>,
}

/// Response for batch project fetching
#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEUnifiedBatchResponse {
    pub results: Vec<super::responses::FEUnifiedSearchResult>,
    pub errors: Vec<String>,
}

#[cfg(test)]
mod task21_filter_tests {
    use super::*;
    fn params(versions: serde_json::Value) -> FEUnifiedSearchParameters {
        serde_json::from_value(serde_json::json!({"gameVersions":versions})).unwrap()
    }
    #[test]
    fn task21_cf_distinct_versions_reject_before_provider_dispatch() {
        let converted: Result<ModSearchParameters, _> =
            params(serde_json::json!(["1.20.1", "1.21.1"])).try_into();
        assert!(
            converted.is_err(),
            "CurseForge supports one Minecraft version; API must not silently truncate"
        );
    }
    #[test]
    fn task21_cf_single_none_and_mr_or_controls() {
        let single: ModSearchParameters = params(serde_json::json!(["1.20.1"])).try_into().unwrap();
        assert_eq!(single.query.game_version.as_deref(), Some("1.20.1"));
        let none: ModSearchParameters = params(serde_json::Value::Null).try_into().unwrap();
        assert_eq!(none.query.game_version, None);
        let mr: ProjectSearchParameters = params(serde_json::json!(["1.20.1", "1.21.1"])).into();
        let facets = serde_json::to_value(mr.facets.unwrap()).unwrap();
        assert_eq!(
            facets,
            serde_json::json!([["versions:1.20.1", "versions:1.21.1"]])
        );
    }
}
