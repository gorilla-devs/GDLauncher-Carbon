//! Models related to search
//! //!
//! [documentation](https://docs.modrinth.com/api-spec/#tag/project_result_model)

use std::{fmt::Display, ops::Deref, str::FromStr};

use crate::domain::modplatforms::modrinth::{
    project::{ProjectSupportRange, ProjectType},
    UtcDateTime,
};
use anyhow::anyhow;
use carbon_macro::into_query_parameters;
use serde::{Deserialize, Serialize};
use url::Url;

use super::version::HashAlgorithm;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ProjectSearchResult {
    /// The slug of a project used for vanity urls.
    pub slug: String,
    /// The title or name of the project
    pub title: String,
    /// A short description of the project
    pub description: String,
    /// A list of the categories that the project has
    pub categories: Option<Vec<String>>,
    /// The client side support of the project
    pub client_side: ProjectSupportRange,
    /// The server side support of the project
    pub server_side: ProjectSupportRange,
    /// The project type of the project
    pub project_type: ProjectType,
    /// The total number of downloads of the project
    pub downloads: u32,
    /// The URL of the project's icon
    pub icon_url: Option<Url>,
    /// The RGB color of the project, automatically generated form the project icon.
    pub color: Option<u32>,
    /// The ID of the project
    pub project_id: String,
    /// The username of the project's author
    pub author: String,
    /// A list of the categories that the project has which are not secondary
    pub display_categories: Option<Vec<String>>,
    /// A list of the minecraft versions supported by the project,
    pub versions: Vec<String>,
    /// The total number of users following the project
    pub follows: u32,
    /// The date the project was added to search
    pub date_created: UtcDateTime,
    /// The date the project was last modified
    pub date_modified: UtcDateTime,
    /// The latest version of minecraft that this project supports
    pub latest_version: Option<String>,
    /// the SPDX license of of a project
    pub license: String,
    /// All gallery images attached to the project
    pub gallery: Option<Vec<String>>,
    /// The featured gallery image of the project
    pub featured_gallery: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum SearchIndex {
    Relevance,
    Downloads,
    Follows,
    Newest,
    Updated,
}

#[derive(Debug, Clone)]
pub enum SearchFacet {
    Category(String),
    Version(String),
    License(String),
    ProjectType(String),
}

impl FromStr for SearchFacet {
    type Err = anyhow::Error;

    fn from_str(facet: &str) -> Result<Self, Self::Err> {
        let Some((facet_type, value)) = facet.trim().split_once(':') else {
            return Err(anyhow!("Improperly formatted search facet `{}`", facet));
        };
        match facet_type {
            "categories" => Ok(SearchFacet::Category(value.to_string())),
            "versions" => Ok(SearchFacet::Version(value.to_string())),
            "license" => Ok(SearchFacet::License(value.to_string())),
            "project_type" => Ok(SearchFacet::ProjectType(value.to_string())),
            _ => Err(anyhow!("Invalid facet type `{}`. Expected one of `categories`, `versions`, `license`, `project_type`", facet_type))
        }
    }
}

impl TryFrom<&str> for SearchFacet {
    type Error = anyhow::Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl Display for SearchFacet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let out = match self {
            SearchFacet::Category(category) => format!("categories:{}", category),
            SearchFacet::Version(version) => format!("version:{}", version),
            SearchFacet::License(license) => format!("license:{}", license),
            SearchFacet::ProjectType(project_type) => format!("project_type:{}", project_type),
        };
        write!(f, "{}", out)
    }
}

impl Serialize for SearchFacet {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for SearchFacet {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SearchFacetOr(Vec<SearchFacet>);

impl From<SearchFacet> for SearchFacetOr {
    fn from(facet: SearchFacet) -> Self {
        SearchFacetOr(vec![facet])
    }
}

impl Deref for SearchFacetOr {
    type Target = Vec<SearchFacet>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for SearchFacetOr {
    type Item = SearchFacet;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<SearchFacet> for SearchFacetOr {
    fn from_iter<I: IntoIterator<Item = SearchFacet>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (size_lower, _) = iter.size_hint();
        let mut c = Vec::with_capacity(size_lower);
        for i in iter {
            c.push(i);
        }
        SearchFacetOr(c)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SearchFacetAnd(Vec<SearchFacetOr>);

impl From<SearchFacetOr> for SearchFacetAnd {
    fn from(facets: SearchFacetOr) -> Self {
        SearchFacetAnd(vec![facets])
    }
}

impl Deref for SearchFacetAnd {
    type Target = Vec<SearchFacetOr>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<SearchFacet> for SearchFacetAnd {
    fn from(facet: SearchFacet) -> Self {
        SearchFacetAnd(vec![facet.into()])
    }
}

impl IntoIterator for SearchFacetAnd {
    type Item = SearchFacetOr;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<SearchFacetOr> for SearchFacetAnd {
    fn from_iter<I: IntoIterator<Item = SearchFacetOr>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (size_lower, _) = iter.size_hint();
        let mut c = Vec::with_capacity(size_lower);
        for i in iter {
            c.push(i);
        }
        SearchFacetAnd(c)
    }
}

#[into_query_parameters]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ProjectSearchParameters {
    /// The query to search for
    /// Example:
    /// ```rust
    /// let query = Some("gravestones".to_string());
    /// ```
    pub query: Option<String>,
    /// The recommended way of filtering search results
    /// see [Search API](https://docs.modrinth.com/docs/tutorials/api_search/)
    /// Example:
    /// ```rust
    /// let facets = Some(SearchFacetAnd(vec![SearchFacet::Category("forge".to_string()).into(), SearchFacet::Version("1.17.1".to_string()).into(),
    ///                   SearchFacet::ProjectType("mod".to_string()).into(), SearchFacetAnd::License("mit".to_string()).into()]));
    /// ```
    pub facets: Option<SearchFacetAnd>,
    /// The sorting method to use for sorting search results.
    /// Default: `Relevance`
    pub index: Option<SearchIndex>,
    /// The offset int he search. Skips this number of results.
    /// Default: 0
    pub offset: Option<u32>,
    /// The number of results returned by the search
    /// Default: 10
    pub limit: Option<u32>,
    /// A list of filters relating to the properties of a project. Use filters when there isn't an
    /// available facet for your needs
    /// PREFER THE USE OF FACETS
    /// Example:
    /// ```rust
    /// let filters = Some("categories=\"fabric\" AND (categories=\"technology\" OR categories=\"utility\")".to_string());
    /// ```
    pub filters: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ProjectSearchResponse {
    /// The List of Results
    pub hits: Vec<ProjectSearchResult>,
    /// The number of results that were skipped by the query
    pub offset: u32,
    /// the number of results that were returned by the query
    pub limit: u32,
    /// the total number of results that match the query
    pub total_hits: u32,
}

#[into_query_parameters]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct VersionIDs {
    /// list of version ids to fetch
    pub ids: Vec<String>,
}

impl Deref for VersionIDs {
    type Target = Vec<String>;
    fn deref(&self) -> &Self::Target {
        &self.ids
    }
}

impl IntoIterator for VersionIDs {
    type Item = String;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.ids.into_iter()
    }
}

impl FromIterator<String> for VersionIDs {
    fn from_iter<T: IntoIterator<Item = String>>(iter: T) -> Self {
        let iter = iter.into_iter();
        let (size_lower, _) = iter.size_hint();
        let mut c = Vec::with_capacity(size_lower);
        for i in iter {
            c.push(i);
        }
        VersionIDs { ids: c }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct VersionHashesQuery {
    /// list of file hashes
    pub hashes: Vec<String>,
    /// algorithm used by hashes
    pub algorithm: HashAlgorithm,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ProjectID(pub String);

impl Deref for ProjectID {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct VersionID(pub String);

impl Deref for VersionID {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[into_query_parameters]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ProjectIDs {
    pub ids: Vec<String>,
}

impl Deref for ProjectIDs {
    type Target = Vec<String>;
    fn deref(&self) -> &Self::Target {
        &self.ids
    }
}

impl IntoIterator for ProjectIDs {
    type Item = String;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.ids.into_iter()
    }
}

impl FromIterator<String> for ProjectIDs {
    fn from_iter<T: IntoIterator<Item = String>>(iter: T) -> Self {
        let iter = iter.into_iter();
        let (size_lower, _) = iter.size_hint();
        let mut c = Vec::with_capacity(size_lower);
        for i in iter {
            c.push(i);
        }
        ProjectIDs { ids: c }
    }
}
