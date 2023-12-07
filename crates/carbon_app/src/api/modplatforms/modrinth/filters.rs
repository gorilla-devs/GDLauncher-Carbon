use std::{
    fmt::Display,
    ops::{Deref, DerefMut},
    str::FromStr,
};

use rspc::Type;
use serde::{Deserialize, Serialize};

use crate::domain::modplatforms::modrinth::{
    project::ProjectVersionsFilters,
    search::{
        ProjectID, ProjectIDs, ProjectSearchParameters, SearchFacet, SearchFacetAnd, SearchFacetOr,
        SearchIndex, TeamID, VersionID, VersionIDs,
    },
};
use anyhow::anyhow;

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum MRFESearchIndex {
    Relevance,
    Downloads,
    Follows,
    Newest,
    Updated,
}

impl From<SearchIndex> for MRFESearchIndex {
    fn from(search_index: SearchIndex) -> Self {
        match search_index {
            SearchIndex::Relevance => MRFESearchIndex::Relevance,
            SearchIndex::Downloads => MRFESearchIndex::Downloads,
            SearchIndex::Follows => MRFESearchIndex::Follows,
            SearchIndex::Newest => MRFESearchIndex::Newest,
            SearchIndex::Updated => MRFESearchIndex::Updated,
        }
    }
}

impl From<MRFESearchIndex> for SearchIndex {
    fn from(search_index: MRFESearchIndex) -> Self {
        match search_index {
            MRFESearchIndex::Relevance => SearchIndex::Relevance,
            MRFESearchIndex::Downloads => SearchIndex::Downloads,
            MRFESearchIndex::Follows => SearchIndex::Follows,
            MRFESearchIndex::Newest => SearchIndex::Newest,
            MRFESearchIndex::Updated => SearchIndex::Updated,
        }
    }
}

#[derive(Type, Debug, Clone)]
pub enum MRFESearchFacet {
    Category(String),
    Version(String),
    License(String),
    ProjectType(String),
}

impl FromStr for MRFESearchFacet {
    type Err = anyhow::Error;

    fn from_str(facet: &str) -> Result<Self, Self::Err> {
        let Some((facet_type, value)) = facet.trim().split_once(':') else {
            return Err(anyhow!("Improperly formatted search facet `{}`", facet));
        };
        match facet_type {
            "categories" => Ok(MRFESearchFacet::Category(value.to_string())),
            "versions" => Ok(MRFESearchFacet::Version(value.to_string())),
            "license" => Ok(MRFESearchFacet::License(value.to_string())),
            "project_type" => Ok(MRFESearchFacet::ProjectType(value.to_string())),
            _ => Err(anyhow!("Invalid facet type `{}`. Expected one of `categories`, `versions`, `license`, `project_type`", facet_type))
        }
    }
}

impl TryFrom<&str> for MRFESearchFacet {
    type Error = anyhow::Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl Display for MRFESearchFacet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let out = match self {
            MRFESearchFacet::Category(category) => format!("categories:{}", category),
            MRFESearchFacet::Version(version) => format!("version:{}", version),
            MRFESearchFacet::License(license) => format!("license:{}", license),
            MRFESearchFacet::ProjectType(project_type) => {
                format!("project_type:{}", project_type)
            }
        };
        write!(f, "{}", out)
    }
}

impl Serialize for MRFESearchFacet {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for MRFESearchFacet {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

impl From<SearchFacet> for MRFESearchFacet {
    fn from(facet: SearchFacet) -> Self {
        match facet {
            SearchFacet::Category(category) => MRFESearchFacet::Category(category),
            SearchFacet::Version(version) => MRFESearchFacet::Version(version),
            SearchFacet::License(license) => MRFESearchFacet::License(license),
            SearchFacet::ProjectType(project_type) => MRFESearchFacet::ProjectType(project_type),
        }
    }
}

impl From<MRFESearchFacet> for SearchFacet {
    fn from(facet: MRFESearchFacet) -> Self {
        match facet {
            MRFESearchFacet::Category(category) => SearchFacet::Category(category),
            MRFESearchFacet::Version(version) => SearchFacet::Version(version),
            MRFESearchFacet::License(license) => SearchFacet::License(license),
            MRFESearchFacet::ProjectType(project_type) => SearchFacet::ProjectType(project_type),
        }
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct MRFESearchFacetOr(pub Vec<MRFESearchFacet>);

impl From<MRFESearchFacet> for MRFESearchFacetOr {
    fn from(facet: MRFESearchFacet) -> Self {
        MRFESearchFacetOr(vec![facet])
    }
}

impl Deref for MRFESearchFacetOr {
    type Target = Vec<MRFESearchFacet>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MRFESearchFacetOr {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl IntoIterator for MRFESearchFacetOr {
    type Item = MRFESearchFacet;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<MRFESearchFacet> for MRFESearchFacetOr {
    fn from_iter<I: IntoIterator<Item = MRFESearchFacet>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (size_lower, _) = iter.size_hint();
        let mut c = Vec::with_capacity(size_lower);
        for i in iter {
            c.push(i);
        }
        MRFESearchFacetOr(c)
    }
}

impl From<SearchFacetOr> for MRFESearchFacetOr {
    fn from(facets: SearchFacetOr) -> Self {
        facets.into_iter().map(Into::into).collect()
    }
}

impl From<MRFESearchFacetOr> for SearchFacetOr {
    fn from(facets: MRFESearchFacetOr) -> Self {
        facets.into_iter().map(Into::into).collect()
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct MRFESearchFacetAnd(pub Vec<MRFESearchFacetOr>);

impl MRFESearchFacetAnd {
    pub fn new() -> Self {
        MRFESearchFacetAnd(Vec::new())
    }
}

impl From<MRFESearchFacetOr> for MRFESearchFacetAnd {
    fn from(facets: MRFESearchFacetOr) -> Self {
        MRFESearchFacetAnd(vec![facets])
    }
}

impl Deref for MRFESearchFacetAnd {
    type Target = Vec<MRFESearchFacetOr>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MRFESearchFacetAnd {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<MRFESearchFacet> for MRFESearchFacetAnd {
    fn from(facet: MRFESearchFacet) -> Self {
        MRFESearchFacetAnd(vec![facet.into()])
    }
}

impl IntoIterator for MRFESearchFacetAnd {
    type Item = MRFESearchFacetOr;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<MRFESearchFacetOr> for MRFESearchFacetAnd {
    fn from_iter<I: IntoIterator<Item = MRFESearchFacetOr>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (size_lower, _) = iter.size_hint();
        let mut c = Vec::with_capacity(size_lower);
        for i in iter {
            c.push(i);
        }
        MRFESearchFacetAnd(c)
    }
}

impl From<SearchFacetAnd> for MRFESearchFacetAnd {
    fn from(facets: SearchFacetAnd) -> Self {
        facets.into_iter().map(Into::into).collect()
    }
}

impl From<MRFESearchFacetAnd> for SearchFacetAnd {
    fn from(facets: MRFESearchFacetAnd) -> Self {
        facets.into_iter().map(Into::into).collect()
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct MRFEProjectSearchParameters {
    pub query: Option<String>,
    pub facets: Option<MRFESearchFacetAnd>,
    pub index: Option<MRFESearchIndex>,
    pub offset: Option<u32>,
    pub limit: Option<u32>,
    pub filters: Option<String>,
}

impl From<ProjectSearchParameters> for MRFEProjectSearchParameters {
    fn from(value: ProjectSearchParameters) -> Self {
        MRFEProjectSearchParameters {
            query: value.query,
            facets: value.facets.map(Into::into),
            index: value.index.map(Into::into),
            offset: value.offset,
            limit: value.limit,
            filters: value.filters,
        }
    }
}

impl From<MRFEProjectSearchParameters> for ProjectSearchParameters {
    fn from(value: MRFEProjectSearchParameters) -> Self {
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
pub struct MRFEProjectVersionsFilters {
    pub project_id: MRFEProjectID,
    #[specta(optional)]
    pub game_version: Option<String>,
    #[specta(optional)]
    pub loaders: Option<String>,
}

impl From<MRFEProjectVersionsFilters> for ProjectVersionsFilters {
    fn from(value: MRFEProjectVersionsFilters) -> Self {
        ProjectVersionsFilters {
            project_id: value.project_id.into(),
            game_versions: value.game_version.into_iter().collect(),
            loaders: value.loaders.into_iter().collect(),
        }
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct MRFEProjectID(pub String);

impl Deref for MRFEProjectID {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<ProjectID> for MRFEProjectID {
    fn from(value: ProjectID) -> Self {
        MRFEProjectID(value.0)
    }
}

impl From<MRFEProjectID> for ProjectID {
    fn from(value: MRFEProjectID) -> Self {
        ProjectID(value.0)
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct MRFETeamID(pub String);

impl Deref for MRFETeamID {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<TeamID> for MRFETeamID {
    fn from(value: TeamID) -> Self {
        MRFETeamID(value.0)
    }
}

impl From<MRFETeamID> for TeamID {
    fn from(value: MRFETeamID) -> Self {
        TeamID(value.0)
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct MRFEVersionID(pub String);

impl Deref for MRFEVersionID {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<VersionID> for MRFEVersionID {
    fn from(value: VersionID) -> Self {
        MRFEVersionID(value.0)
    }
}

impl From<MRFEVersionID> for VersionID {
    fn from(value: MRFEVersionID) -> Self {
        VersionID(value.0)
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct MRFEProjectIDs(pub Vec<String>);

impl Deref for MRFEProjectIDs {
    type Target = Vec<String>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MRFEProjectIDs {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl IntoIterator for MRFEProjectIDs {
    type Item = String;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<String> for MRFEProjectIDs {
    fn from_iter<T: IntoIterator<Item = String>>(iter: T) -> Self {
        let iter = iter.into_iter();
        let (size_lower, _) = iter.size_hint();
        let mut c = Vec::with_capacity(size_lower);
        for i in iter {
            c.push(i);
        }
        MRFEProjectIDs(c)
    }
}

impl From<ProjectIDs> for MRFEProjectIDs {
    fn from(value: ProjectIDs) -> Self {
        MRFEProjectIDs(value.ids)
    }
}

impl From<MRFEProjectIDs> for ProjectIDs {
    fn from(value: MRFEProjectIDs) -> Self {
        ProjectIDs { ids: value.0 }
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct MRFEVersionIDs(pub Vec<String>);

impl Deref for MRFEVersionIDs {
    type Target = Vec<String>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MRFEVersionIDs {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl IntoIterator for MRFEVersionIDs {
    type Item = String;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<String> for MRFEVersionIDs {
    fn from_iter<T: IntoIterator<Item = String>>(iter: T) -> Self {
        let iter = iter.into_iter();
        let (size_lower, _) = iter.size_hint();
        let mut c = Vec::with_capacity(size_lower);
        for i in iter {
            c.push(i);
        }
        MRFEVersionIDs(c)
    }
}

impl From<VersionIDs> for MRFEVersionIDs {
    fn from(value: VersionIDs) -> Self {
        MRFEVersionIDs(value.ids)
    }
}

impl From<MRFEVersionIDs> for VersionIDs {
    fn from(value: MRFEVersionIDs) -> Self {
        VersionIDs { ids: value.0 }
    }
}
