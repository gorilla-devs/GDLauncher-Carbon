use std::fmt::Display;
use std::ops::{Deref, DerefMut};

use rspc::Type;
use serde::{Deserialize, Serialize};

use super::curseforge::filters::FEModSearchParametersQuery;
use super::modrinth;
use super::{curseforge, FESearchAPI};

#[derive(Type, Debug, Deserialize, Serialize)]
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

#[derive(Type, Debug, Deserialize, Serialize)]
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

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FEUnifiedModSortIndex {
    CurseForge(curseforge::filters::FEModSearchSortField),
    Modrinth(modrinth::filters::FEModrinthSearchIndex),
}

#[derive(Type, Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum FEQueryModLoaderType {
    Forge,
    Fabric,
    Quilt,
}

impl Display for FEQueryModLoaderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let out = match self {
            FEQueryModLoaderType::Forge => "forge",
            FEQueryModLoaderType::Fabric => "fabric",
            FEQueryModLoaderType::Quilt => "quilt",
        };
        write!(f, "{}", out)
    }
}

impl From<FEQueryModLoaderType> for curseforge::structs::FEModLoaderType {
    fn from(value: FEQueryModLoaderType) -> Self {
        match value {
            FEQueryModLoaderType::Forge => curseforge::structs::FEModLoaderType::Forge,
            FEQueryModLoaderType::Fabric => curseforge::structs::FEModLoaderType::Fabric,
            FEQueryModLoaderType::Quilt => curseforge::structs::FEModLoaderType::Quilt,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FEUnifiedSearchType {
    Mod,
    ModPack,
}

impl Display for FEUnifiedSearchType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let out = match self {
            FEUnifiedSearchType::Mod => "mod",
            FEUnifiedSearchType::ModPack => "modpack",
        };
        write!(f, "{}", out)
    }
}

impl From<FEUnifiedSearchType> for curseforge::structs::FEClassId {
    fn from(value: FEUnifiedSearchType) -> Self {
        match value {
            FEUnifiedSearchType::Mod => curseforge::structs::FEClassId::Mods,
            FEUnifiedSearchType::ModPack => curseforge::structs::FEClassId::Modpacks,
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FEUnifiedSearchCategoryID {
    Curseforge(i32),
    Modrinth(String),
}

#[derive(Type, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEUnifiedSearchParameters {
    pub search_query: Option<String>,
    pub categories: Option<And<FEUnifiedSearchCategoryID>>,
    pub game_versions: Option<Or<String>>,
    pub modloaders: Option<Or<FEQueryModLoaderType>>,
    pub project_type: Option<FEUnifiedSearchType>,
    pub sort_index: Option<FEUnifiedModSortIndex>,
    pub sort_order: Option<curseforge::filters::FEModSearchSortOrder>,
    pub index: Option<u32>,
    pub page_size: Option<u32>,
    pub search_api: FESearchAPI,
}

impl TryFrom<FEUnifiedSearchParameters> for curseforge::filters::FEModSearchParameters {
    type Error = anyhow::Error;

    fn try_from(value: FEUnifiedSearchParameters) -> Result<Self, Self::Error> {
        Ok(curseforge::filters::FEModSearchParameters {
            query: FEModSearchParametersQuery {
                game_id: 432,
                search_filter: value.search_query,
                game_version: value.game_versions.and_then(|vers| vers.into_iter().next()),
                category_id: value.categories.and_then(|cat_groups| {
                    cat_groups.into_iter().next().and_then(|cats| {
                        cats.into_iter().find_map(|cat| match cat {
                            FEUnifiedSearchCategoryID::Curseforge(id) => Some(id),
                            FEUnifiedSearchCategoryID::Modrinth(_) => None,
                        })
                    })
                }),
                sort_order: value.sort_order,
                sort_field: match value.sort_index {
                    Some(FEUnifiedModSortIndex::CurseForge(field)) => Some(field),
                    _ => None,
                },
                class_id: value.project_type.map(Into::into),
                mod_loader_type: value
                    .modloaders
                    .and_then(|loaders| loaders.into_iter().next().map(Into::into)),
                author_id: None,
                game_version_type_id: None,
                slug: None,
                index: value.index.map(|index| index as i32),
                page_size: value.page_size.map(|page_size| page_size as i32),
            },
        })
    }
}

impl TryFrom<FEUnifiedSearchParameters>
    for crate::domain::modplatforms::curseforge::filters::ModSearchParameters
{
    type Error = anyhow::Error;

    fn try_from(value: FEUnifiedSearchParameters) -> Result<Self, Self::Error> {
        let search_params: curseforge::filters::FEModSearchParameters = value.try_into()?;
        Ok(search_params.into())
    }
}

impl TryFrom<FEUnifiedSearchParameters> for modrinth::filters::FEModrinthProjectSearchParameters {
    type Error = anyhow::Error;
    fn try_from(value: FEUnifiedSearchParameters) -> Result<Self, Self::Error> {
        let mut facets = modrinth::filters::FEModrinthSearchFacetAnd::new();
        if let Some(categories) = value.categories {
            for cat_or in categories {
                let category_or = cat_or
                    .into_iter()
                    .filter_map(|cat| match cat {
                        FEUnifiedSearchCategoryID::Curseforge(_) => None,
                        FEUnifiedSearchCategoryID::Modrinth(id) => {
                            Some(modrinth::filters::FEModrinthSearchFacet::Category(id))
                        }
                    })
                    .collect();
                facets.push(category_or);
            }
        }
        if let Some(versions) = value.game_versions {
            let versions_or = versions
                .into_iter()
                .map(modrinth::filters::FEModrinthSearchFacet::Version)
                .collect();
            facets.push(versions_or);
        }
        if let Some(modloaders) = value.modloaders {
            let modloaders_or = modloaders
                .into_iter()
                .map(|modloader| {
                    modrinth::filters::FEModrinthSearchFacet::Category(modloader.to_string())
                })
                .collect();
            facets.push(modloaders_or);
        }
        if let Some(project_type) = value.project_type {
            facets.push(modrinth::filters::FEModrinthSearchFacetOr(vec![
                modrinth::filters::FEModrinthSearchFacet::ProjectType(project_type.to_string()),
            ]));
        }
        Ok(modrinth::filters::FEModrinthProjectSearchParameters {
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
        })
    }
}
