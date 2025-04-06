use super::curseforge::filters::CFFEModSearchParametersQuery;
use super::unified::{FEUnifiedCategoryId, FEUnifiedModLoaderType};
use super::{curseforge, FESearchAPI};
use super::{modrinth, FEUnifiedSearchType};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::fmt::Display;
use std::ops::{Deref, DerefMut};

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
pub struct And<T>(pub Vec<Or<T>>);

impl<T> Deref for And<T> {
    type Target = Vec<Or<T>>;

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
    type Item = Or<T>;
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
            c.push(Or(vec![i]));
        }
        And(c)
    }
}

impl<T> FromIterator<Or<T>> for And<T> {
    fn from_iter<I: IntoIterator<Item = Or<T>>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (size_lower, _) = iter.size_hint();
        let mut c = Vec::with_capacity(size_lower);
        for i in iter {
            c.push(i);
        }
        And(c)
    }
}

impl<T> From<Or<T>> for And<T> {
    fn from(value: Or<T>) -> Self {
        And(vec![value])
    }
}

#[derive(Type, Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum FEUnifiedEnvironment {
    Server,
    Client,
}

#[derive(Type, Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum FEUnifiedModSortIndex {
    CurseForge(curseforge::filters::CFFEModSearchSortField),
    Modrinth(modrinth::filters::MRFESearchIndex),
}

#[derive(Type, Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FEUnifiedSearchParameters {
    pub search_query: Option<String>,
    pub categories: Option<And<FEUnifiedCategoryId>>,
    pub game_versions: Option<Or<String>>,
    pub modloaders: Option<Or<FEUnifiedModLoaderType>>,
    pub project_type: Option<FEUnifiedSearchType>,
    pub sort_index: Option<FEUnifiedModSortIndex>,
    pub sort_order: Option<curseforge::filters::CFFEModSearchSortOrder>,
    pub index: Option<u32>,
    pub page_size: Option<u32>,
    pub search_api: Option<FESearchAPI>,
    pub environment: Option<FEUnifiedEnvironment>,
}

impl From<FEUnifiedSearchParameters> for curseforge::filters::CFFEModSearchParameters {
    fn from(value: FEUnifiedSearchParameters) -> Self {
        curseforge::filters::CFFEModSearchParameters {
            query: CFFEModSearchParametersQuery {
                game_id: 432,
                search_filter: value.search_query,
                game_version: value.game_versions.and_then(|vers| vers.into_iter().next()),
                category_ids: value.categories.map(|cat_groups| {
                    cat_groups
                        .into_iter()
                        .filter_map(|cats| {
                            // Curseforge does't support ORs of categories, take only the first of each
                            // group
                            cats.into_iter().find_map(|cat| match cat {
                                FEUnifiedCategoryId::Curseforge(id) => Some(id),
                                FEUnifiedCategoryId::Modrinth(_) => None,
                            })
                        })
                        .collect()
                }),
                sort_order: value.sort_order,
                sort_field: match value.sort_index {
                    Some(FEUnifiedModSortIndex::CurseForge(field)) => Some(field),
                    _ => None,
                },
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
        }
    }
}

impl TryFrom<FEUnifiedSearchParameters>
    for carbon_platforms::curseforge::filters::ModSearchParameters
{
    type Error = anyhow::Error;

    fn try_from(value: FEUnifiedSearchParameters) -> Result<Self, Self::Error> {
        let search_params: curseforge::filters::CFFEModSearchParameters = value.try_into()?;
        Ok(search_params.into())
    }
}

impl From<FEUnifiedSearchParameters> for modrinth::filters::MRFEProjectSearchParameters {
    fn from(value: FEUnifiedSearchParameters) -> Self {
        let mut facets = modrinth::filters::MRFESearchFacetAnd::new();
        if let Some(categories) = value.categories {
            for cat_or in categories {
                let category_or = cat_or
                    .into_iter()
                    .filter_map(|cat| match cat {
                        FEUnifiedCategoryId::Curseforge(_) => None,
                        FEUnifiedCategoryId::Modrinth(id) => {
                            Some(modrinth::filters::MRFESearchFacet::Category(id))
                        }
                    })
                    .collect();
                facets.push(category_or);
            }
        }
        if let Some(versions) = value.game_versions {
            let versions_or = versions
                .into_iter()
                .map(modrinth::filters::MRFESearchFacet::Version)
                .collect();
            facets.push(versions_or);
        }
        if let Some(modloaders) = value.modloaders {
            let modloaders_or = modloaders
                .into_iter()
                .filter_map(|loader| {
                    TryInto::<modrinth::structs::MRFELoaderType>::try_into(loader).ok()
                })
                .map(|modloader| {
                    modrinth::filters::MRFESearchFacet::Category(modloader.to_string())
                })
                .collect();
            facets.push(modloaders_or);
        }
        if let Some(project_type) = value.project_type {
            facets.push(modrinth::filters::MRFESearchFacetOr(vec![
                modrinth::filters::MRFESearchFacet::ProjectType(project_type.to_string()),
            ]));
        }
        modrinth::filters::MRFEProjectSearchParameters {
            query: value.search_query,
            facets: if facets.is_empty() {
                None
            } else {
                Some(facets)
            },
            index: match value.sort_index {
                Some(FEUnifiedModSortIndex::Modrinth(index)) => Some(index),
                _ => None,
            },
            offset: value.index,
            limit: value.page_size,
            filters: None,
        }
    }
}
