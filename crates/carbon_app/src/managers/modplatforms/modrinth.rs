use std::collections::HashMap;

use anyhow::Ok;
use reqwest_middleware::ClientWithMiddleware;
use tracing::trace;
use url::Url;

use crate::{
    domain::modplatforms::modrinth::{
        project::Project,
        responses::{
            CategoriesResponse, ProjectsResponse, TeamResponse, VersionHashesResponse,
            VersionsResponse, LoadersResponse,
        },
        search::{
            ProjectID, ProjectIDs, ProjectSearchParameters, ProjectSearchResponse, TeamID,
            VersionHashesQuery, VersionID, VersionIDs,
        },
        version::Version,
    },
    error::request::RequestError,
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
    pub async fn get_categories(&self) -> anyhow::Result<CategoriesResponse> {
        let url = self.base_url.join("tag/category")?;

        trace!("GET {}", url);

        let body = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .error_for_status()
            .map_err(RequestError::from_error)?
            .text()
            .await
            .map_err(RequestError::from_error)?;
        let categories = serde_json::from_str(&body)
            .map_err(|err| RequestError::from_json_decode_error(err, &body, &url))?;
        Ok(categories)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_loaders(&self) -> anyhow::Result<LoadersResponse> {
        let url = self.base_url.join("tag/loader")?;

        trace!("GET {}", url);

        let body = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .error_for_status()
            .map_err(RequestError::from_error)?
            .text()
            .await
            .map_err(RequestError::from_error)?;
        let categories = serde_json::from_str(&body)
            .map_err(|err| RequestError::from_json_decode_error(err, &body, &url))?;
        Ok(categories)
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

        let body = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .error_for_status()
            .map_err(RequestError::from_error)?
            .text()
            .await
            .map_err(RequestError::from_error)?;
        let search_results = serde_json::from_str(&body)
            .map_err(|err| RequestError::from_json_decode_error(err, &body, &url))?;
        Ok(search_results)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_project(&self, project: ProjectID) -> anyhow::Result<Project> {
        let url = self.base_url.join(&format!("project/{}", &*project))?;

        trace!("GET {}", url);

        let body = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .error_for_status()
            .map_err(RequestError::from_error)?
            .text()
            .await
            .map_err(RequestError::from_error)?;
        let proj = serde_json::from_str(&body)
            .map_err(|err| RequestError::from_json_decode_error(err, &body, &url))?;
        Ok(proj)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_projects(&self, projects: ProjectIDs) -> anyhow::Result<ProjectsResponse> {
        let mut url = self.base_url.join("projects")?;
        let query = projects.into_query_parameters()?;
        url.set_query(Some(&query));

        trace!("GET {}", url);

        let body = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .error_for_status()
            .map_err(RequestError::from_error)?
            .text()
            .await
            .map_err(RequestError::from_error)?;
        let projects: ProjectsResponse = serde_json::from_str(&body)
            .map_err(|err| RequestError::from_json_decode_error(err, &body, &url))?;
        Ok(projects)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_version(&self, version: VersionID) -> anyhow::Result<Version> {
        let url = self.base_url.join(&format!("version/{}", &*version))?;

        trace!("GET {}", url);

        let body = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .error_for_status()
            .map_err(RequestError::from_error)?
            .text()
            .await
            .map_err(RequestError::from_error)?;
        let ver: Version = serde_json::from_str(&body)
            .map_err(|err| RequestError::from_json_decode_error(err, &body, &url))?;
        Ok(ver)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_versions(&self, version_ids: VersionIDs) -> anyhow::Result<VersionsResponse> {
        let mut url = self.base_url.join("versions")?;
        let query = version_ids.into_query_parameters()?;
        url.set_query(Some(&query));

        trace!("GET {}", url);

        let body = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .error_for_status()
            .map_err(RequestError::from_error)?
            .text()
            .await
            .map_err(RequestError::from_error)?;
        let versions: VersionsResponse = serde_json::from_str(&body)
            .map_err(|err| RequestError::from_json_decode_error(err, &body, &url))?;
        Ok(versions)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_versions_from_hash(
        &self,
        hashes_query: &VersionHashesQuery,
    ) -> anyhow::Result<VersionHashesResponse> {
        let url = self.base_url.join("version_files")?;

        let body = serde_json::to_string(hashes_query)?;

        trace!("POST {url} - {body:?}");

        let body = self
            .client
            .post(url.as_str())
            .json(&hashes_query)
            .send()
            .await?
            .error_for_status()
            .map_err(RequestError::from_error)?
            .text()
            .await
            .map_err(RequestError::from_error)?;
        let versions: VersionHashesResponse = serde_json::from_str(&body)
            .map_err(|err| RequestError::from_json_decode_error(err, &body, &url))?;
        Ok(versions)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_team(&self, team: TeamID) -> anyhow::Result<TeamResponse> {
        let url = self.base_url.join(&format!("team/{}/members", &*team))?;

        trace!("GET {}", url);

        let body = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .error_for_status()
            .map_err(RequestError::from_error)?
            .text()
            .await
            .map_err(RequestError::from_error)?;
        let team = serde_json::from_str::<TeamResponse>(&body)
            .map_err(|err| RequestError::from_json_decode_error(err, &body, &url))?
            .into_iter()
            .filter(|member| member.accepted)
            .collect::<TeamResponse>();
        Ok(team)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_project_team(&self, project: ProjectID) -> anyhow::Result<TeamResponse> {
        let url = self
            .base_url
            .join(&format!("project/{}/members", &*project))?;

        trace!("GET {}", url);

        let body = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .error_for_status()
            .map_err(RequestError::from_error)?
            .text()
            .await
            .map_err(RequestError::from_error)?;
        let team = serde_json::from_str::<TeamResponse>(&body)
            .map_err(|err| RequestError::from_json_decode_error(err, &body, &url))?
            .into_iter()
            .filter(|member| member.accepted)
            .collect::<TeamResponse>();
        Ok(team)
    }
}

#[cfg(test)]
mod test {
    use tracing_test::traced_test;

    use crate::domain::modplatforms::modrinth::{
        search::{SearchFacet, SearchIndex},
        version::HashAlgorithm,
    };

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
    async fn test_search_with_query() -> anyhow::Result<()> {
        use super::*;

        let client = reqwest::Client::builder().build().unwrap();
        let client = reqwest_middleware::ClientBuilder::new(client).build();
        let modrinth = Modrinth::new(client);

        let facets = vec![
            SearchFacet::Category("forge".to_string()),
            SearchFacet::Version("1.17.1".to_string()),
        ];

        let search_params = ProjectSearchParameters {
            query: Some("jei".to_string()),
            facets: Some(facets.into_iter().map(Into::into).collect()),
            index: Some(SearchIndex::Updated),
            offset: None,
            limit: None,
            filters: None,
        };

        let facets_json = serde_json::to_string(&search_params.facets)?;
        tracing::info!("Search Facet's string: {:?}", facets_json);

        tracing::info!("Modrinth Search params are: {:?}", search_params);

        let query = search_params.into_query_parameters()?;
        tracing::info!("URL query is: {:?}", query);

        let results = modrinth.search(search_params).await?;
        tracing::info!("Modrinth Search results are: {:?}", results);
        assert!(!results.hits.is_empty());
        Ok(())
    }

    #[tokio::test]
    #[traced_test]
    async fn test_fetch_categories() {
        use super::*;

        let client = reqwest::Client::builder().build().unwrap();
        let client = reqwest_middleware::ClientBuilder::new(client).build();
        let modrinth = Modrinth::new(client);

        let results = modrinth.get_categories().await.unwrap();
        tracing::debug!("Categories: {:?}", results);
        assert!(!results.is_empty());
    }


    #[tokio::test]
    #[traced_test]
    async fn test_fetch_loaders() {
        use super::*;

        let client = reqwest::Client::builder().build().unwrap();
        let client = reqwest_middleware::ClientBuilder::new(client).build();
        let modrinth = Modrinth::new(client);

        let results = modrinth.get_loaders().await.unwrap();
        tracing::debug!("Modloaders: {:?}", results);
        assert!(!results.is_empty());
    }

    #[tokio::test]
    #[traced_test]
    async fn test_get_project() -> anyhow::Result<()> {
        use super::*;

        let client = reqwest::Client::builder().build()?;
        let client = reqwest_middleware::ClientBuilder::new(client).build();
        let modrinth = Modrinth::new(client);

        let result = modrinth
            .get_project(ProjectID("u6dRKJwZ".to_string()))
            .await?;
        tracing::debug!("Project: {:?}", result);
        assert!(result.id == "u6dRKJwZ");
        assert!(result.title == "Just Enough Items");
        Ok(())
    }

    #[tokio::test]
    #[traced_test]
    async fn test_get_project_team() -> anyhow::Result<()> {
        use super::*;

        let client = reqwest::Client::builder().build()?;
        let client = reqwest_middleware::ClientBuilder::new(client).build();
        let modrinth = Modrinth::new(client);

        let results = modrinth
            .get_project_team(ProjectID("u6dRKJwZ".to_string()))
            .await?;
        tracing::debug!("Project Team: {:?}", results);
        assert!(!results.is_empty());
        Ok(())
    }

    #[tokio::test]
    #[traced_test]
    async fn test_get_team() -> anyhow::Result<()> {
        use super::*;

        let client = reqwest::Client::builder().build()?;
        let client = reqwest_middleware::ClientBuilder::new(client).build();
        let modrinth = Modrinth::new(client);

        let results = modrinth.get_team(TeamID("SfcwZ8an".to_string())).await?;
        tracing::debug!("Team: {:?}", results);
        assert!(!results.is_empty());
        Ok(())
    }

    #[tokio::test]
    #[traced_test]
    async fn test_get_version() -> anyhow::Result<()> {
        use super::*;

        let client = reqwest::Client::builder().build()?;
        let client = reqwest_middleware::ClientBuilder::new(client).build();
        let modrinth = Modrinth::new(client);

        let result = modrinth
            .get_version(VersionID("6QsZu0uX".to_string()))
            .await?;
        tracing::debug!("Version: {:?}", result);
        assert!(result.project_id == "u6dRKJwZ");
        assert!(result.name == "1.0.1 for 1.8");
        Ok(())
    }

    #[tokio::test]
    #[traced_test]
    async fn test_get_versions_from_hash() -> anyhow::Result<()> {
        use super::*;

        let client = reqwest::Client::builder().build()?;
        let client = reqwest_middleware::ClientBuilder::new(client).build();
        let modrinth = Modrinth::new(client);

        let results = modrinth
            .get_versions_from_hash(&VersionHashesQuery {
                hashes: vec!["09b63cb3bf2bf6ea89967684d352f58f7951b242".to_string()],
                algorithm: HashAlgorithm::SHA1,
            })
            .await?;
        tracing::debug!("Versions: {:?}", results);
        assert!(!results.is_empty());
        let result = results
            .get(&"09b63cb3bf2bf6ea89967684d352f58f7951b242".to_string())
            .ok_or_else(|| anyhow::anyhow!("Hash not found"))?;
        assert!(result.project_id == "u6dRKJwZ");
        assert!(result.name == "1.0.1 for 1.8");
        Ok(())
    }
}
