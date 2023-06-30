use std::collections::HashMap;

use anyhow::Ok;
use reqwest_middleware::ClientWithMiddleware;
use tracing::trace;
use url::Url;

use crate::domain::modplatforms::modrinth::{
    project::Project,
    search::{ProjectSearchParameters, ProjectSearchResponse, VersionHashesQuery, VersionsQuery},
    tag::Category,
    version::Version,
};

pub struct Modrinth {
    client: ClientWithMiddleware,
    base_url: Url,
}

pub const MODRINTH_API_BASE: &str = "https://api.modrinth.com/v2/";

impl Modrinth {
    pub fn new(client: reqwest_middleware::ClientWithMiddleware) -> Self {
        let base_url = String::from(MODRINTH_API_BASE);
        Self {
            client,
            base_url: base_url.parse().expect("Invalid base URL"),
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_categories(&self) -> anyhow::Result<Vec<Category>> {
        let url = self.base_url.join("/tag/category")?;

        trace!("GET {}", url);

        let resp = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .json::<Vec<Category>>()
            .await?;
        Ok(resp)
    }

    #[tracing::instrument(skip(self))]
    pub async fn search(
        &self,
        search_params: ProjectSearchParameters,
    ) -> anyhow::Result<ProjectSearchResponse> {
        let mut url = self.base_url.join("search")?;
        let query = search_params.into_query_parameters()?;
        url.set_query(Some(&query));

        trace!("GET {}", url);

        let resp = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .json::<ProjectSearchResponse>()
            .await?;
        Ok(resp)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_project(&self, project_id: &str) -> anyhow::Result<Project> {
        let url = self.base_url.join("project")?.join(project_id)?;

        trace!("GET {}", url);

        let resp = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .json::<Project>()
            .await?;
        Ok(resp)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_version(&self, version_id: &str) -> anyhow::Result<Version> {
        let url = self.base_url.join("version")?.join(version_id)?;

        trace!("GET {}", url);

        let resp = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .json::<Version>()
            .await?;
        Ok(resp)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_versions(&self, version_ids: &VersionsQuery) -> anyhow::Result<Vec<Version>> {
        let mut url = self.base_url.join("versions")?;
        let query = version_ids.into_query_parameters()?;
        url.set_query(Some(&query));

        trace!("GET {}", url);

        let resp = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .json::<Vec<Version>>()
            .await?;
        Ok(resp)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_versions_from_hash(
        &self,
        hashes_query: &VersionHashesQuery,
    ) -> anyhow::Result<HashMap<String, Version>> {
        let url = self.base_url.join("version_files")?;

        let body = serde_json::to_string(hashes_query)?;

        trace!("POST {url} - {body:?}");

        let resp = self
            .client
            .post(url.as_str())
            .body(body)
            .send()
            .await?
            .json::<HashMap<String, Version>>()
            .await?;
        Ok(resp)
    }
}

#[cfg(test)]
mod test {
    use tracing_test::traced_test;

    #[tokio::test]
    #[traced_test]
    async fn test_search_no_query() {
        use super::*;

        let client = reqwest::Client::builder().build().unwrap();
        let client = reqwest_middleware::ClientBuilder::new(client).build();
        let modrinth = Modrinth::new(client);

        let search_params = ProjectSearchParameters {
            query: None,
            facets: None,
            index: None,
            offset: None,
            limit: None,
            filters: None,
        };

        let results = modrinth.search(search_params).await.unwrap();
        assert!(!results.hits.is_empty());
    }

    #[tokio::test]
    #[traced_test]
    async fn test_search_with_query() {
        use super::*;

        let client = reqwest::Client::builder().build().unwrap();
        let client = reqwest_middleware::ClientBuilder::new(client).build();
        let modrinth = Modrinth::new(client);

        let search_params = ProjectSearchParameters {
            query: Some("jei".to_string()),
            facets: None,
            index: None,
            offset: None,
            limit: None,
            filters: None,
        };

        let results = modrinth.search(search_params).await.unwrap();
        assert!(!results.hits.is_empty());
    }
}
