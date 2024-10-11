use std::{collections::HashSet, sync::Arc};

use anyhow::Ok;
use reqwest_middleware::ClientWithMiddleware;
use tracing::trace;
use url::Url;

use crate::{
    domain::{
        instance::info::{ModLoader, ModLoaderType, StandardVersion},
        modplatforms::modrinth::{
            project::{Project, ProjectVersionsFilters},
            responses::{
                CategoriesResponse, LoadersResponse, ProjectsResponse, TeamResponse,
                VersionHashesResponse, VersionsResponse,
            },
            search::{
                ProjectID, ProjectIDs, ProjectSearchParameters, ProjectSearchResponse, TeamID,
                TeamIDs, VersionHashesQuery, VersionID, VersionIDs,
            },
            version::{ModrinthPackDependencies, Version},
        },
    },
    error::request::GoodJsonRequestError,
    managers::AppInner,
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

        let categories = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .json_with_context_reporting("modrinth::get_categories")
            .await?;

        Ok(categories)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_loaders(&self) -> anyhow::Result<LoadersResponse> {
        let url = self.base_url.join("tag/loader")?;

        trace!("GET {}", url);

        let categories = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .json_with_context_reporting("modrinth::get_loaders")
            .await?;
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

        let search_results = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .json_with_context_reporting("modrinth::search")
            .await?;
        Ok(search_results)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_project(&self, project: ProjectID) -> anyhow::Result<Project> {
        let url = self.base_url.join(&format!("project/{}", &*project))?;

        trace!("GET {}", url);

        let proj = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .json_with_context_reporting("modrinth::get_project")
            .await?;
        Ok(proj)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_project_versions(
        &self,
        filters: ProjectVersionsFilters,
    ) -> anyhow::Result<VersionsResponse> {
        let mut url = self
            .base_url
            .join(&format!("project/{}/version", &*filters.project_id))?;

        let query = filters.into_query_parameters()?;
        url.set_query(Some(&query));

        trace!("GET {}", url);

        let proj = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .json_with_context_reporting("modrinth::get_project_versions")
            .await?;
        Ok(proj)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_projects(&self, projects: ProjectIDs) -> anyhow::Result<ProjectsResponse> {
        let mut url = self.base_url.join("projects")?;
        let query = projects.into_query_parameters()?;
        url.set_query(Some(&query));

        trace!("GET {}", url);

        let projects = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .json_with_context_reporting("modrinth::get_projects")
            .await?;
        Ok(projects)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_version(&self, version: VersionID) -> anyhow::Result<Version> {
        let url = self.base_url.join(&format!("version/{}", &*version))?;

        trace!("GET {}", url);

        let ver = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .json_with_context_reporting("modrinth::get_version")
            .await?;
        Ok(ver)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_versions(&self, version_ids: VersionIDs) -> anyhow::Result<VersionsResponse> {
        let mut url = self.base_url.join("versions")?;
        let query = version_ids.into_query_parameters()?;
        url.set_query(Some(&query));

        trace!("GET {}", url);

        let versions = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .json_with_context_reporting("modrinth::get_versions")
            .await?;
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

        let versions = self
            .client
            .post(url.as_str())
            .body(reqwest::Body::from(serde_json::to_string(&hashes_query)?))
            .send()
            .await?
            .json_with_context_reporting("modrinth::get_versions_from_hash")
            .await?;
        Ok(versions)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_team(&self, team: TeamID) -> anyhow::Result<TeamResponse> {
        let url = self.base_url.join(&format!("team/{}/members", &*team))?;

        trace!("GET {}", url);

        let team = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .json_with_context_reporting::<TeamResponse>("modrinth::get_team")
            .await?
            .into_iter()
            .filter(|member| member.accepted)
            .collect::<TeamResponse>();
        Ok(team)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_teams(&self, team_ids: TeamIDs) -> anyhow::Result<Vec<TeamResponse>> {
        let mut url = self.base_url.join("teams")?;
        let query = team_ids.into_query_parameters()?;
        url.set_query(Some(&query));

        trace!("GET {}", url);

        let teams = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .json_with_context_reporting::<Vec<TeamResponse>>("modrinth::get_teams")
            .await?
            .into_iter()
            .map(|team| team.into_iter().filter(|member| member.accepted).collect())
            .collect::<Vec<TeamResponse>>();
        Ok(teams)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_project_team(&self, project: ProjectID) -> anyhow::Result<TeamResponse> {
        let url = self
            .base_url
            .join(&format!("project/{}/members", &*project))?;

        trace!("GET {}", url);

        let team = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .json_with_context_reporting::<TeamResponse>("modrinth::get_project_team")
            .await?
            .into_iter()
            .filter(|member| member.accepted)
            .collect::<TeamResponse>();
        Ok(team)
    }
}

pub async fn convert_mr_version_to_standard_version(
    app: Arc<AppInner>,
    modrinth_version: ModrinthPackDependencies,
) -> anyhow::Result<StandardVersion> {
    let minecraft_version = modrinth_version
        .minecraft
        .ok_or_else(|| anyhow::anyhow!("Modpack does not have a Minecraft version listed"))?;

    let mut modloaders = HashSet::new();
    if let Some(forge_version) = modrinth_version.forge {
        let forge_manifest = app.minecraft_manager().get_forge_manifest().await?;

        let forge_version = forge_manifest
            .game_versions
            .into_iter()
            .find(|v| v.id == minecraft_version)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "forge manifest does not contain version '{}'",
                    minecraft_version
                )
            })?
            .loaders
            .into_iter()
            .find(|l| l.id.contains(&forge_version))
            .ok_or_else(|| {
                anyhow::anyhow!("forge manifest does not contain loader '{}'", forge_version)
            })?;

        modloaders.insert(ModLoader {
            type_: ModLoaderType::Forge,
            version: forge_version.id,
        });
    }
    if let Some(fabric_version) = modrinth_version.fabric_loader {
        modloaders.insert(ModLoader {
            type_: ModLoaderType::Fabric,
            version: fabric_version,
        });
    }
    if let Some(quilt_version) = modrinth_version.quilt_loader {
        modloaders.insert(ModLoader {
            type_: ModLoaderType::Quilt,
            version: quilt_version,
        });
    }
    if let Some(neoforge_version) = modrinth_version.neoforge {
        modloaders.insert(ModLoader {
            type_: ModLoaderType::Neoforge,
            version: neoforge_version,
        });
    }

    let gdl_version = StandardVersion {
        release: minecraft_version,
        modloaders,
    };

    Ok(gdl_version)
}

pub fn convert_standard_version_to_mr_version(
    standard_version: StandardVersion,
) -> ModrinthPackDependencies {
    let mut modrinth_version = ModrinthPackDependencies {
        minecraft: Some(standard_version.release),
        forge: None,
        fabric_loader: None,
        quilt_loader: None,
        neoforge: None,
    };

    for modloader in standard_version.modloaders {
        match modloader.type_ {
            ModLoaderType::Forge => {
                modrinth_version.forge = Some(modloader.version);
            }
            ModLoaderType::Fabric => {
                modrinth_version.fabric_loader = Some(modloader.version);
            }
            ModLoaderType::Quilt => {
                modrinth_version.quilt_loader = Some(modloader.version);
            }
            ModLoaderType::Neoforge => {
                modrinth_version.neoforge = Some(modloader.version);
            }
        }
    }

    modrinth_version
}

#[cfg(test)]
mod test {
    use tracing_test::traced_test;

    use crate::{
        domain::modplatforms::modrinth::{
            search::{SearchFacet, SearchIndex},
            version::HashAlgorithm,
        },
        iridium_client,
    };

    #[tokio::test]
    #[traced_test]
    async fn test_search_no_query() {
        use super::*;

        let client = iridium_client::get_client(env!("BASE_API").to_string()).build();
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

        let client = iridium_client::get_client(env!("BASE_API").to_string()).build();
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

        let client = iridium_client::get_client(env!("BASE_API").to_string()).build();
        let modrinth = Modrinth::new(client);

        let results = modrinth.get_categories().await.unwrap();
        tracing::debug!("Categories: {:?}", results);
        assert!(!results.is_empty());
    }

    #[tokio::test]
    #[traced_test]
    async fn test_fetch_loaders() {
        use super::*;

        let client = iridium_client::get_client(env!("BASE_API").to_string()).build();
        let modrinth = Modrinth::new(client);

        let results = modrinth.get_loaders().await.unwrap();
        tracing::debug!("Modloaders: {:?}", results);
        assert!(!results.is_empty());
    }

    #[tokio::test]
    #[traced_test]
    async fn test_get_project() -> anyhow::Result<()> {
        use super::*;
        let client = iridium_client::get_client(env!("BASE_API").to_string()).build();
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
    async fn test_get_project_versions() -> anyhow::Result<()> {
        use super::*;
        let client = iridium_client::get_client(env!("BASE_API").to_string()).build();
        let modrinth = Modrinth::new(client);

        let results = modrinth
            .get_project_versions(ProjectVersionsFilters {
                project_id: ProjectID("u6dRKJwZ".to_string()),
                game_versions: None,
                loaders: None,
                offset: None,
                limit: None,
            })
            .await?;
        tracing::debug!("Versions: {:?}", results);
        assert!(!results.is_empty());
        Ok(())
    }

    #[tokio::test]
    #[traced_test]
    async fn test_get_project_team() -> anyhow::Result<()> {
        use super::*;
        let client = iridium_client::get_client(env!("BASE_API").to_string()).build();
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
        let client = iridium_client::get_client(env!("BASE_API").to_string()).build();
        let modrinth = Modrinth::new(client);

        let results = modrinth.get_team(TeamID("SfcwZ8an".to_string())).await?;
        tracing::debug!("Team: {:?}", results);
        assert!(!results.is_empty());
        Ok(())
    }

    #[tokio::test]
    #[traced_test]
    async fn test_get_teams() -> anyhow::Result<()> {
        use super::*;
        let client = iridium_client::get_client(env!("BASE_API").to_string()).build();
        let modrinth = Modrinth::new(client);

        let results = modrinth
            .get_teams(TeamIDs {
                ids: vec!["SfcwZ8an".to_string(), "BZoBsPo6".to_string()],
            })
            .await?;
        tracing::debug!("Teams: {:?}", results);
        assert!(!results.is_empty());
        Ok(())
    }

    #[tokio::test]
    #[traced_test]
    async fn test_get_version() -> anyhow::Result<()> {
        use super::*;
        let client = iridium_client::get_client(env!("BASE_API").to_string()).build();
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

        let client = iridium_client::get_client(env!("BASE_API").to_string()).build();
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
