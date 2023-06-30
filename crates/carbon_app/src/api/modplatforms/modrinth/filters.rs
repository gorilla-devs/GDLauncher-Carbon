use std::{fmt::Display, ops::Deref, str::FromStr};

use carbon_macro::into_query_parameters;
use rspc::Type;
use serde::{Deserialize, Serialize};

use crate::domain::modplatforms::modrinth::search::{
    ProjectSearchParameters, SearchFacet, SearchFacetAnd, SearchFacetOr, SearchIndex,
};
use anyhow::anyhow;

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum FESearchIndex {
    Relevance,
    Downloads,
    Follows,
    Newest,
    Updated,
}

impl From<SearchIndex> for FESearchIndex {
    fn from(search_index: SearchIndex) -> Self {
        match search_index {
            SearchIndex::Relevance => FESearchIndex::Relevance,
            SearchIndex::Downloads => FESearchIndex::Downloads,
            SearchIndex::Follows => FESearchIndex::Follows,
            SearchIndex::Newest => FESearchIndex::Newest,
            SearchIndex::Updated => FESearchIndex::Updated,
        }
    }
}

impl From<FESearchIndex> for SearchIndex {
    fn from(search_index: FESearchIndex) -> Self {
        match search_index {
            FESearchIndex::Relevance => SearchIndex::Relevance,
            FESearchIndex::Downloads => SearchIndex::Downloads,
            FESearchIndex::Follows => SearchIndex::Follows,
            FESearchIndex::Newest => SearchIndex::Newest,
            FESearchIndex::Updated => SearchIndex::Updated,
        }
    }
}

#[derive(Type, Debug, Clone)]
pub enum FESearchFacet {
    Category(String),
    Version(String),
    License(String),
    ProjectType(String),
}

impl FromStr for FESearchFacet {
    type Err = anyhow::Error;

    fn from_str(facet: &str) -> Result<Self, Self::Err> {
        let Some((facet_type, value)) = facet.trim().split_once(':') else {
            return Err(anyhow!("Improperly formatted search facet `{}`", facet));
        };
        match facet_type {
            "categories" => Ok(FESearchFacet::Category(value.to_string())),
            "versions" => Ok(FESearchFacet::Version(value.to_string())),
            "license" => Ok(FESearchFacet::License(value.to_string())),
            "project_type" => Ok(FESearchFacet::ProjectType(value.to_string())),
            _ => Err(anyhow!("Invalid facet type `{}`. Expected one of `categories`, `versions`, `license`, `project_type`", facet_type))
        }
    }
}

impl TryFrom<&str> for FESearchFacet {
    type Error = anyhow::Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl Display for FESearchFacet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let out = match self {
            FESearchFacet::Category(category) => format!("categories:{}", category),
            FESearchFacet::Version(version) => format!("version:{}", version),
            FESearchFacet::License(license) => format!("license:{}", license),
            FESearchFacet::ProjectType(project_type) => format!("project_type:{}", project_type),
        };
        write!(f, "{}", out)
    }
}

impl Serialize for FESearchFacet {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for FESearchFacet {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

impl From<SearchFacet> for FESearchFacet {
    fn from(facet: SearchFacet) -> Self {
        match facet {
            SearchFacet::Category(category) => FESearchFacet::Category(category),
            SearchFacet::Version(version) => FESearchFacet::Version(version),
            SearchFacet::License(license) => FESearchFacet::License(license),
            SearchFacet::ProjectType(project_type) => FESearchFacet::ProjectType(project_type),
        }
    }
}

impl From<FESearchFacet> for SearchFacet {
    fn from(facet: FESearchFacet) -> Self {
        match facet {
            FESearchFacet::Category(category) => SearchFacet::Category(category),
            FESearchFacet::Version(version) => SearchFacet::Version(version),
            FESearchFacet::License(license) => SearchFacet::License(license),
            FESearchFacet::ProjectType(project_type) => SearchFacet::ProjectType(project_type),
        }
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct FESearchFacetOr(Vec<FESearchFacet>);

impl From<FESearchFacet> for FESearchFacetOr {
    fn from(facet: FESearchFacet) -> Self {
        FESearchFacetOr(vec![facet])
    }
}

impl Deref for FESearchFacetOr {
    type Target = Vec<FESearchFacet>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for FESearchFacetOr {
    type Item = FESearchFacet;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<FESearchFacet> for FESearchFacetOr {
    fn from_iter<I: IntoIterator<Item = FESearchFacet>>(iter: I) -> Self {
        let mut c = Vec::new();
        for i in iter {
            c.push(i);
        }
        FESearchFacetOr(c)
    }
}

impl From<SearchFacetOr> for FESearchFacetOr {
    fn from(facets: SearchFacetOr) -> Self {
        FESearchFacetOr::from_iter(facets.into_iter().map(|facet| facet.into()))
    }
}

impl From<FESearchFacetOr> for SearchFacetOr {
    fn from(facets: FESearchFacetOr) -> Self {
        SearchFacetOr::from_iter(facets.into_iter().map(|facet| facet.into()))
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct FESearchFacetAnd(Vec<FESearchFacetOr>);

impl From<FESearchFacetOr> for FESearchFacetAnd {
    fn from(facets: FESearchFacetOr) -> Self {
        FESearchFacetAnd(vec![facets])
    }
}

impl Deref for FESearchFacetAnd {
    type Target = Vec<FESearchFacetOr>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<FESearchFacet> for FESearchFacetAnd {
    fn from(facet: FESearchFacet) -> Self {
        FESearchFacetAnd(vec![facet.into()])
    }
}

impl IntoIterator for FESearchFacetAnd {
    type Item = FESearchFacetOr;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<FESearchFacetOr> for FESearchFacetAnd {
    fn from_iter<I: IntoIterator<Item = FESearchFacetOr>>(iter: I) -> Self {
        let mut c = Vec::new();
        for i in iter {
            c.push(i);
        }
        FESearchFacetAnd(c)
    }
}

impl From<SearchFacetAnd> for FESearchFacetAnd {
    fn from(facets: SearchFacetAnd) -> Self {
        FESearchFacetAnd::from_iter(facets.into_iter().map(|facet| facet.into()))
    }
}

impl From<FESearchFacetAnd> for SearchFacetAnd {
    fn from(facets: FESearchFacetAnd) -> Self {
        SearchFacetAnd::from_iter(facets.into_iter().map(|facet| facet.into()))
    }
}

#[into_query_parameters]
#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct FEProjectSearchParameters {
    pub query: Option<String>,
    pub facets: Option<FESearchFacetAnd>,
    pub index: Option<FESearchIndex>,
    pub offset: Option<u32>,
    pub limit: Option<u32>,
    pub filters: Option<String>,
}

impl From<ProjectSearchParameters> for FEProjectSearchParameters {
    fn from(value: ProjectSearchParameters) -> Self {
        FEProjectSearchParameters {
            query: value.query,
            facets: value.facets.map(|facets| facets.into()),
            index: value.index.map(|index| index.into()),
            offset: value.offset,
            limit: value.limit,
            filters: value.filters,
        }
    }
}

impl From<FEProjectSearchParameters> for ProjectSearchParameters {
    fn from(value: FEProjectSearchParameters) -> Self {
        ProjectSearchParameters {
            query: value.query,
            facets: value.facets.map(|facets| facets.into()),
            index: value.index.map(|index| index.into()),
            offset: value.offset,
            limit: value.limit,
            filters: value.filters,
        }
    }
}
