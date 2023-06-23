//! Models related to search
//! //!
//! [documentation](https://docs.modrinth.com/api-spec/#tag/project_result_model)

use crate::domain::modplatforms::modrinth::{
    project::{ProjectSupportRange, ProjectType},
    ArcStr, Number, UtcDateTime,
};
use carbon_macro::into_query_parameters;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SearchResult {
    /// The slug of a project used for vanity urls.
    pub slug: ArcStr,
    /// The title or name of the project
    pub title: ArcStr,
    /// A short description of the project
    pub description: String,
    /// A list of the categories that the project has
    pub categories: Option<Vec<ArcStr>>,
    /// The client side support of the project
    pub client_side: ProjectSupportRange,
    /// The server side support of the project
    pub server_side: ProjectSupportRange,
    /// The project type of the project
    pub project_type: ProjectType,
    /// The total number of downloads of the project
    pub downloads: Number,
    /// The URL of the project's icon
    pub icon_url: Option<Url>,
    /// The RGB color of the project, automatically generated form the project icon.
    pub color: Option<Number>,
    /// The ID of the project
    pub project_id: ArcStr,
    /// The username of the project's author
    pub author: ArcStr,
    /// A list of the categories that the project has which are not secondary
    pub display_categories: Option<Vec<ArcStr>>,
    /// A list of the minecraft versions supported by the project,
    pub versions: Vec<ArcStr>,
    /// The total number of users following the project
    pub follows: Number,
    /// The date the project was added to search
    pub date_created: UtcDateTime,
    /// The date the project was last modified
    pub date_modified: UtcDateTime,
    /// The latest version of minecraft that this project supports
    pub latest_version: Option<ArcStr>,
    /// the SPDX license of of a project
    pub license: ArcStr,
    /// All gallery images attached to the project
    pub gallery: Option<Vec<ArcStr>>,
    /// The featured gallery image of the project
    pub featured_gallery: Option<ArcStr>,
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

#[into_query_parameters]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SearchParameters {
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
    /// let facets = Some(vec!["categories:forge".to_string(), "versions:1.17.1".to_string(),
    ///                   "project_type:mod".to_string(), "license:mit".to_string(),]);
    /// ```
    pub facets: Option<Vec<String>>,
    /// The sorting method to use for sorting search results.
    /// Default: `Relevance`
    pub index: Option<SearchIndex>,
    /// The offset int he search. Skips this number of results.
    /// Default: 0
    pub offset: Option<Number>,
    /// The number of results returned by the search
    /// Default: 10
    pub limit: Option<Number>,
    /// A list of filters relating to the properties of a project. Use filters when there isn't an
    /// available facet for your needs
    /// Example:
    /// ```rust
    /// let filters = Some("categories=\"fabric\" AND (categories=\"technology\" OR categories=\"utility\")".to_string());
    /// ```
    pub filters: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SearchResponse {
    /// The List of Results
    pub hits: Vec<SearchResult>,
    /// The number of results that were skipped by the query
    pub offset: Number,
    /// the number of results that were returned by the query
    pub limit: Number,
    /// the total number of results that match the query
    pub total_hits: Number,
}
