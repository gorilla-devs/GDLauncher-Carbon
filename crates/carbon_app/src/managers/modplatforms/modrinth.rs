use crate::{
    domain::instance::info::{ModLoader, ModLoaderType, StandardVersion},
    error::request::GoodJsonRequestError,
    managers::AppInner,
};
use anyhow::Ok;
use carbon_platforms::modrinth::{
    project::{Project, ProjectVersionsFilters},
    responses::{
        CategoriesResponse, LoadersResponse, ProjectsResponse, TeamResponse, VersionHashesResponse,
        VersionsResponse,
    },
    search::{
        ProjectID, ProjectIDs, ProjectSearchParameters, ProjectSearchResponse, TeamID, TeamIDs,
        VersionHashesQuery, VersionID, VersionIDs,
    },
    version::{LatestVersionsBody, ModrinthPackDependencies, Version, VersionType},
};
use reqwest_middleware::ClientWithMiddleware;
use std::{collections::HashSet, sync::Arc};
use tracing::trace;
use url::Url;

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

    /// Points the client at a stand-in for the API, so request shape can be
    /// asserted without depending on live data.
    #[cfg(test)]
    fn with_base_url(client: reqwest_middleware::ClientWithMiddleware, base_url: &str) -> Self {
        Self {
            client,
            base_url: base_url.parse().expect("Invalid base URL"),
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_categories(&self) -> anyhow::Result<CategoriesResponse> {
        let url = self.base_url.join("tag/category")?;

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

        // HashMap-ordered ID lists produce different URLs for the same data,
        // making the URL-keyed HTTP cache balloon without ever hitting. Skip it.
        let projects = self
            .client
            .get(url.as_str())
            .header("avoid-caching", "")
            .send()
            .await?
            .json_with_context_reporting("modrinth::get_projects")
            .await?;
        Ok(projects)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_version(&self, version: VersionID) -> anyhow::Result<Version> {
        let url = self.base_url.join(&format!("version/{}", &*version))?;

        let ver = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .json_with_context_reporting("modrinth::get_version")
            .await?;
        Ok(ver)
    }

    #[tracing::instrument(skip(self, version_ids))]
    pub async fn get_versions(&self, version_ids: VersionIDs) -> anyhow::Result<VersionsResponse> {
        let mut url = self.base_url.join("versions")?;
        let query = version_ids.into_query_parameters()?;
        url.set_query(Some(&query));

        // HashMap-ordered ID lists produce different URLs for the same data,
        // making the URL-keyed HTTP cache balloon without ever hitting. Skip it.
        let versions = self
            .client
            .get(url.as_str())
            .header("avoid-caching", "")
            .send()
            .await?
            .json_with_context_reporting("modrinth::get_versions")
            .await?;
        Ok(versions)
    }

    #[tracing::instrument(skip(self, hashes_query))]
    pub async fn get_versions_from_hash(
        &self,
        hashes_query: &VersionHashesQuery,
    ) -> anyhow::Result<VersionHashesResponse> {
        let url = self.base_url.join("version_files")?;

        let body = serde_json::to_string(hashes_query)?;

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

    /// Returns the newest version matching the given loaders and game versions for
    /// each hash, letting the server answer "is there an update" in a single request
    /// instead of the client fetching every version a project has ever published.
    #[tracing::instrument(skip(self))]
    pub async fn get_latest_versions_from_hashes(
        &self,
        body: &LatestVersionsBody,
    ) -> anyhow::Result<VersionHashesResponse> {
        let url = self.base_url.join("version_files/update")?;

        let versions = self
            .client
            .post(url.as_str())
            .body(reqwest::Body::from(serde_json::to_string(body)?))
            .send()
            .await?
            .json_with_context_reporting("modrinth::get_latest_versions_from_hashes")
            .await?;
        Ok(versions)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_team(&self, team: TeamID) -> anyhow::Result<TeamResponse> {
        let url = self.base_url.join(&format!("team/{}/members", &*team))?;

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

        // HashMap-ordered ID lists produce different URLs for the same data,
        // making the URL-keyed HTTP cache balloon without ever hitting. Skip it.
        let teams = self
            .client
            .get(url.as_str())
            .header("avoid-caching", "")
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
    use crate::iridium_client;
    use carbon_platforms::modrinth::{
        search::{SearchFacet, SearchIndex},
        version::HashAlgorithm,
    };
    use tracing_test::traced_test;

    #[tokio::test]
    #[traced_test]
    async fn test_search_no_query() {
        use super::*;

        let client = iridium_client::get_client(crate::util::base_api::get_base_api_env!()).build();
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

        let client = iridium_client::get_client(crate::util::base_api::get_base_api_env!()).build();
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

        let client = iridium_client::get_client(crate::util::base_api::get_base_api_env!()).build();
        let modrinth = Modrinth::new(client);

        let results = modrinth.get_categories().await.unwrap();
        tracing::debug!("Categories: {:?}", results);
        assert!(!results.is_empty());
    }

    #[tokio::test]
    #[traced_test]
    async fn test_fetch_loaders() {
        use super::*;

        let client = iridium_client::get_client(crate::util::base_api::get_base_api_env!()).build();
        let modrinth = Modrinth::new(client);

        let results = modrinth.get_loaders().await.unwrap();
        tracing::debug!("Modloaders: {:?}", results);
        assert!(!results.is_empty());
    }

    #[tokio::test]
    #[traced_test]
    async fn test_get_project() -> anyhow::Result<()> {
        use super::*;
        let client = iridium_client::get_client(crate::util::base_api::get_base_api_env!()).build();
        let modrinth = Modrinth::new(client);

        let result = modrinth
            .get_project(ProjectID("u6dRKJwZ".to_string()))
            .await?;
        tracing::debug!("Project: {:?}", result);
        assert!(result.id == "u6dRKJwZ");
        assert!(result.title == "Just Enough Items (JEI)");
        Ok(())
    }

    #[tokio::test]
    #[traced_test]
    async fn test_get_project_versions() -> anyhow::Result<()> {
        use super::*;
        let client = iridium_client::get_client(crate::util::base_api::get_base_api_env!()).build();
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
        let client = iridium_client::get_client(crate::util::base_api::get_base_api_env!()).build();
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
        let client = iridium_client::get_client(crate::util::base_api::get_base_api_env!()).build();
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
        let client = iridium_client::get_client(crate::util::base_api::get_base_api_env!()).build();
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
        let client = iridium_client::get_client(crate::util::base_api::get_base_api_env!()).build();
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

        let client = iridium_client::get_client(crate::util::base_api::get_base_api_env!()).build();
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

    #[tokio::test]
    #[traced_test]
    async fn test_get_latest_versions_from_hashes() -> anyhow::Result<()> {
        use super::*;

        let client = iridium_client::get_client(crate::util::base_api::get_base_api_env!()).build();
        let modrinth = Modrinth::new(client);

        // Sodium 0.4.2 for 1.19.x; newer builds for the same loaders and game
        // versions exist, so the response must point at a later version.
        let hash = "95589fcca80f77aca8e38634927bfb7a5bd5b31b7f34c09352cc7724541b9efe\
                    8bbe1d7c1a39afcdbf67fa38f5871355ccb56817027bf6028255393c7174e450"
            .to_string();
        let installed = modrinth
            .get_versions_from_hash(&VersionHashesQuery {
                hashes: vec![hash.clone()],
                algorithm: HashAlgorithm::SHA512,
            })
            .await?;

        let installed = installed
            .get(&hash)
            .ok_or_else(|| anyhow::anyhow!("Hash not found"))?;

        let latest = modrinth
            .get_latest_versions_from_hashes(&LatestVersionsBody {
                hashes: vec![hash.clone()],
                algorithm: HashAlgorithm::SHA512,
                loaders: installed.loaders.clone(),
                game_versions: installed.game_versions.clone(),
                version_types: None,
            })
            .await?;

        let latest = latest
            .get(&hash)
            .ok_or_else(|| anyhow::anyhow!("No latest version returned for hash"))?;

        assert_eq!(latest.project_id, installed.project_id);
        assert_ne!(
            latest.id, installed.id,
            "a newer version exists, so the update endpoint must not return the installed one"
        );
        assert!(latest.date_published > installed.date_published);
        Ok(())
    }

    /// The channel filter is only useful if it actually reaches the wire, and a
    /// serialisation change would drop it silently. Uses a stand-in for the API
    /// so it keeps holding regardless of what any project publishes.
    #[tokio::test]
    async fn test_version_types_is_sent_on_the_wire() -> anyhow::Result<()> {
        use super::*;

        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/version_files/update")
            .match_body(mockito::Matcher::PartialJson(serde_json::json!({
                "version_types": ["beta"],
                "loaders": ["forge"],
                "game_versions": ["1.20.1"],
            })))
            .with_status(200)
            .with_body("{}")
            .expect(1)
            .create_async()
            .await;

        let client = iridium_client::get_client(crate::util::base_api::get_base_api_env!()).build();
        let modrinth = Modrinth::with_base_url(client, &format!("{}/", server.url()));

        modrinth
            .get_latest_versions_from_hashes(&LatestVersionsBody {
                hashes: vec!["hash".to_string()],
                algorithm: HashAlgorithm::SHA512,
                loaders: vec!["forge".to_string()],
                game_versions: vec!["1.20.1".to_string()],
                version_types: Some(vec![VersionType::Beta]),
            })
            .await?;

        mock.assert_async().await;
        Ok(())
    }

    /// An unset filter has to be left out of the body rather than sent as null,
    /// which the API rejects.
    #[test]
    fn test_absent_version_types_is_omitted_from_the_body() {
        use super::*;

        let body = LatestVersionsBody {
            hashes: vec!["hash".to_string()],
            algorithm: HashAlgorithm::SHA512,
            loaders: vec!["forge".to_string()],
            game_versions: vec!["1.20.1".to_string()],
            version_types: None,
        };

        let serialized = serde_json::to_string(&body).unwrap();

        assert!(
            !serialized.contains("version_types"),
            "unset channel filter must not appear in the body: {serialized}"
        );
    }

    /// `version_types` is absent from Modrinth's published schema for this route
    /// even though the server implements it, so update paths would silently lose
    /// their per channel meaning if the API ever stopped honouring it.
    #[tokio::test]
    #[traced_test]
    async fn test_get_latest_versions_from_hashes_honours_version_types() -> anyhow::Result<()> {
        use super::*;

        let client = iridium_client::get_client(crate::util::base_api::get_base_api_env!()).build();
        let modrinth = Modrinth::new(client);

        // Autochef's Delight for Forge 1.20.1: its newest build is a pre-release
        // and its newest stable one is older, so an ignored filter cannot produce
        // the same answer for both channels. Dormant since 2025, and versions are
        // not unpublished, so both channels keep resolving.
        let hash = "81fa997c75fbd524fcbdad6731a99b702de1a66f6c2b25d4df1dd9fc24bb74e6\
                    96cb4000019a61812d77e59674784b3b1270e203e7e8dc16c4992eceb0278b5c"
            .to_string();

        let query = async |version_type: VersionType| {
            modrinth
                .get_latest_versions_from_hashes(&LatestVersionsBody {
                    hashes: vec![hash.clone()],
                    algorithm: HashAlgorithm::SHA512,
                    loaders: vec!["forge".to_string()],
                    game_versions: vec!["1.20.1".to_string()],
                    version_types: Some(vec![version_type]),
                })
                .await
        };

        let stable = query(VersionType::Release).await?;
        let stable = stable
            .get(&hash)
            .ok_or_else(|| anyhow::anyhow!("no stable build returned"))?;

        let prerelease = query(VersionType::Beta).await?;
        let prerelease = prerelease
            .get(&hash)
            .ok_or_else(|| anyhow::anyhow!("no pre-release build returned"))?;

        assert_eq!(
            stable.version_type,
            VersionType::Release,
            "asked for stable builds, got {:?}",
            stable.version_type
        );
        assert_eq!(
            prerelease.version_type,
            VersionType::Beta,
            "asked for pre-release builds, got {:?}",
            prerelease.version_type
        );
        assert_ne!(
            stable.id, prerelease.id,
            "both channels resolved to the same version, so the filter was ignored"
        );

        Ok(())
    }
}
