use std::{fmt::Display, ops::Deref, str::FromStr};

use rspc::Type;
use serde::{Deserialize, Serialize};

use crate::domain::modplatforms::modrinth::search::{
    ProjectID, ProjectIDs, ProjectSearchParameters, SearchFacet, SearchFacetAnd, SearchFacetOr,
    SearchIndex, VersionID, VersionIDs,
};
use anyhow::anyhow;

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum FEModrinthSearchIndex {
    Relevance,
    Downloads,
    Follows,
    Newest,
    Updated,
}

impl From<SearchIndex> for FEModrinthSearchIndex {
    fn from(search_index: SearchIndex) -> Self {
        match search_index {
            SearchIndex::Relevance => FEModrinthSearchIndex::Relevance,
            SearchIndex::Downloads => FEModrinthSearchIndex::Downloads,
            SearchIndex::Follows => FEModrinthSearchIndex::Follows,
            SearchIndex::Newest => FEModrinthSearchIndex::Newest,
            SearchIndex::Updated => FEModrinthSearchIndex::Updated,
        }
    }
}

impl From<FEModrinthSearchIndex> for SearchIndex {
    fn from(search_index: FEModrinthSearchIndex) -> Self {
        match search_index {
            FEModrinthSearchIndex::Relevance => SearchIndex::Relevance,
            FEModrinthSearchIndex::Downloads => SearchIndex::Downloads,
            FEModrinthSearchIndex::Follows => SearchIndex::Follows,
            FEModrinthSearchIndex::Newest => SearchIndex::Newest,
            FEModrinthSearchIndex::Updated => SearchIndex::Updated,
        }
    }
}

#[derive(Type, Debug, Clone)]
pub enum FEModrinthSearchFacet {
    Category(String),
    Version(String),
    License(String),
    ProjectType(String),
}

impl FromStr for FEModrinthSearchFacet {
    type Err = anyhow::Error;

    fn from_str(facet: &str) -> Result<Self, Self::Err> {
        let Some((facet_type, value)) = facet.trim().split_once(':') else {
            return Err(anyhow!("Improperly formatted search facet `{}`", facet));
        };
        match facet_type {
            "categories" => Ok(FEModrinthSearchFacet::Category(value.to_string())),
            "versions" => Ok(FEModrinthSearchFacet::Version(value.to_string())),
            "license" => Ok(FEModrinthSearchFacet::License(value.to_string())),
            "project_type" => Ok(FEModrinthSearchFacet::ProjectType(value.to_string())),
            _ => Err(anyhow!("Invalid facet type `{}`. Expected one of `categories`, `versions`, `license`, `project_type`", facet_type))
        }
    }
}

impl TryFrom<&str> for FEModrinthSearchFacet {
    type Error = anyhow::Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl Display for FEModrinthSearchFacet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let out = match self {
            FEModrinthSearchFacet::Category(category) => format!("categories:{}", category),
            FEModrinthSearchFacet::Version(version) => format!("version:{}", version),
            FEModrinthSearchFacet::License(license) => format!("license:{}", license),
            FEModrinthSearchFacet::ProjectType(project_type) => {
                format!("project_type:{}", project_type)
            }
        };
        write!(f, "{}", out)
    }
}

impl Serialize for FEModrinthSearchFacet {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for FEModrinthSearchFacet {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

impl From<SearchFacet> for FEModrinthSearchFacet {
    fn from(facet: SearchFacet) -> Self {
        match facet {
            SearchFacet::Category(category) => FEModrinthSearchFacet::Category(category),
            SearchFacet::Version(version) => FEModrinthSearchFacet::Version(version),
            SearchFacet::License(license) => FEModrinthSearchFacet::License(license),
            SearchFacet::ProjectType(project_type) => {
                FEModrinthSearchFacet::ProjectType(project_type)
            }
        }
    }
}

impl From<FEModrinthSearchFacet> for SearchFacet {
    fn from(facet: FEModrinthSearchFacet) -> Self {
        match facet {
            FEModrinthSearchFacet::Category(category) => SearchFacet::Category(category),
            FEModrinthSearchFacet::Version(version) => SearchFacet::Version(version),
            FEModrinthSearchFacet::License(license) => SearchFacet::License(license),
            FEModrinthSearchFacet::ProjectType(project_type) => {
                SearchFacet::ProjectType(project_type)
            }
        }
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct FEModrinthSearchFacetOr(pub Vec<FEModrinthSearchFacet>);

impl From<FEModrinthSearchFacet> for FEModrinthSearchFacetOr {
    fn from(facet: FEModrinthSearchFacet) -> Self {
        FEModrinthSearchFacetOr(vec![facet])
    }
}

impl Deref for FEModrinthSearchFacetOr {
    type Target = Vec<FEModrinthSearchFacet>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for FEModrinthSearchFacetOr {
    type Item = FEModrinthSearchFacet;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<FEModrinthSearchFacet> for FEModrinthSearchFacetOr {
    fn from_iter<I: IntoIterator<Item = FEModrinthSearchFacet>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (size_lower, _) = iter.size_hint();
        let mut c = Vec::with_capacity(size_lower);
        for i in iter {
            c.push(i);
        }
        FEModrinthSearchFacetOr(c)
    }
}

impl From<SearchFacetOr> for FEModrinthSearchFacetOr {
    fn from(facets: SearchFacetOr) -> Self {
        facets.into_iter().map(Into::into).collect()
    }
}

impl From<FEModrinthSearchFacetOr> for SearchFacetOr {
    fn from(facets: FEModrinthSearchFacetOr) -> Self {
        facets.into_iter().map(Into::into).collect()
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct FEModrinthSearchFacetAnd(pub Vec<FEModrinthSearchFacetOr>);

impl From<FEModrinthSearchFacetOr> for FEModrinthSearchFacetAnd {
    fn from(facets: FEModrinthSearchFacetOr) -> Self {
        FEModrinthSearchFacetAnd(vec![facets])
    }
}

impl Deref for FEModrinthSearchFacetAnd {
    type Target = Vec<FEModrinthSearchFacetOr>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<FEModrinthSearchFacet> for FEModrinthSearchFacetAnd {
    fn from(facet: FEModrinthSearchFacet) -> Self {
        FEModrinthSearchFacetAnd(vec![facet.into()])
    }
}

impl IntoIterator for FEModrinthSearchFacetAnd {
    type Item = FEModrinthSearchFacetOr;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<FEModrinthSearchFacetOr> for FEModrinthSearchFacetAnd {
    fn from_iter<I: IntoIterator<Item = FEModrinthSearchFacetOr>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (size_lower, _) = iter.size_hint();
        let mut c = Vec::with_capacity(size_lower);
        for i in iter {
            c.push(i);
        }
        FEModrinthSearchFacetAnd(c)
    }
}

impl From<SearchFacetAnd> for FEModrinthSearchFacetAnd {
    fn from(facets: SearchFacetAnd) -> Self {
        facets.into_iter().map(Into::into).collect()
    }
}

impl From<FEModrinthSearchFacetAnd> for SearchFacetAnd {
    fn from(facets: FEModrinthSearchFacetAnd) -> Self {
        facets.into_iter().map(Into::into).collect()
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct FEModrinthProjectSearchParameters {
    pub query: Option<String>,
    pub facets: Option<FEModrinthSearchFacetAnd>,
    pub index: Option<FEModrinthSearchIndex>,
    pub offset: Option<u32>,
    pub limit: Option<u32>,
    pub filters: Option<String>,
}

impl From<ProjectSearchParameters> for FEModrinthProjectSearchParameters {
    fn from(value: ProjectSearchParameters) -> Self {
        FEModrinthProjectSearchParameters {
            query: value.query,
            facets: value.facets.map(Into::into),
            index: value.index.map(Into::into),
            offset: value.offset,
            limit: value.limit,
            filters: value.filters,
        }
    }
}

impl From<FEModrinthProjectSearchParameters> for ProjectSearchParameters {
    fn from(value: FEModrinthProjectSearchParameters) -> Self {
        ProjectSearchParameters {
            query: value.query,
            facets: value.facets.map(Into::into),
            index: value.index.map(Into::into),
            offset: value.offset,
            limit: value.limit,
            filters: value.filters,
        }
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct FEModrinthProjectID(pub String);

impl Deref for FEModrinthProjectID {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<ProjectID> for FEModrinthProjectID {
    fn from(value: ProjectID) -> Self {
        FEModrinthProjectID(value.0)
    }
}

impl From<FEModrinthProjectID> for ProjectID {
    fn from(value: FEModrinthProjectID) -> Self {
        ProjectID(value.0)
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct FEModrinthVersionID(pub String);

impl Deref for FEModrinthVersionID {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<VersionID> for FEModrinthVersionID {
    fn from(value: VersionID) -> Self {
        FEModrinthVersionID(value.0)
    }
}

impl From<FEModrinthVersionID> for VersionID {
    fn from(value: FEModrinthVersionID) -> Self {
        VersionID(value.0)
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct FEModrinthProjectIDs(pub Vec<String>);

impl Deref for FEModrinthProjectIDs {
    type Target = Vec<String>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for FEModrinthProjectIDs {
    type Item = String;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<String> for FEModrinthProjectIDs {
    fn from_iter<T: IntoIterator<Item = String>>(iter: T) -> Self {
        let iter = iter.into_iter();
        let (size_lower, _) = iter.size_hint();
        let mut c = Vec::with_capacity(size_lower);
        for i in iter {
            c.push(i);
        }
        FEModrinthProjectIDs(c)
    }
}

impl From<ProjectIDs> for FEModrinthProjectIDs {
    fn from(value: ProjectIDs) -> Self {
        FEModrinthProjectIDs(value.ids)
    }
}

impl From<FEModrinthProjectIDs> for ProjectIDs {
    fn from(value: FEModrinthProjectIDs) -> Self {
        ProjectIDs { ids: value.0 }
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct FEModrinthVersionIDs(pub Vec<String>);

impl Deref for FEModrinthVersionIDs {
    type Target = Vec<String>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for FEModrinthVersionIDs {
    type Item = String;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<String> for FEModrinthVersionIDs {
    fn from_iter<T: IntoIterator<Item = String>>(iter: T) -> Self {
        let iter = iter.into_iter();
        let (size_lower, _) = iter.size_hint();
        let mut c = Vec::with_capacity(size_lower);
        for i in iter {
            c.push(i);
        }
        FEModrinthVersionIDs(c)
    }
}

impl From<VersionIDs> for FEModrinthVersionIDs {
    fn from(value: VersionIDs) -> Self {
        FEModrinthVersionIDs(value.ids)
    }
}

impl From<FEModrinthVersionIDs> for VersionIDs {
    fn from(value: FEModrinthVersionIDs) -> Self {
        VersionIDs { ids: value.0 }
    }
}
