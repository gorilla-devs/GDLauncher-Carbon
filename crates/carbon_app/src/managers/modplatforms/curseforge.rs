use std::sync::Arc;

use anyhow::bail;
use reqwest_middleware::ClientWithMiddleware;
use serde_json::json;
use tracing::{info, trace};
use url::Url;

use crate::{
    domain::{
        self,
        instance::info::{ModLoader, ModLoaderType, StandardVersion},
        modplatforms::curseforge::{
            filters::{
                FilesParameters, ModDescriptionParameters, ModFileChangelogParameters,
                ModFileParameters, ModFilesParameters, ModParameters, ModSearchParameters,
                ModsParameters,
            },
            Category, CurseForgeResponse, File, FingerprintsMatchesResult, MinecraftModLoaderIndex,
            Mod,
        },
    },
    error::request::GoodJsonRequestError,
    managers::{AppInner, GDL_API_BASE},
};

pub struct CurseForge {
    client: ClientWithMiddleware,
    base_url: Url,
}

impl CurseForge {
    pub fn new(client: reqwest_middleware::ClientWithMiddleware) -> Self {
        let base_url = format!("{GDL_API_BASE}/v1/curseforge/");
        Self {
            client,
            base_url: base_url.parse().unwrap(),
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_modloaders(
        &self,
    ) -> anyhow::Result<CurseForgeResponse<Vec<MinecraftModLoaderIndex>>> {
        let url = self.base_url.join("minecraft/modloader")?;

        trace!("GET {}", url);

        let resp = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .json_with_context_reporting::<CurseForgeResponse<Vec<MinecraftModLoaderIndex>>>(
                "curseforge::get_modloaders",
            )
            .await?;

        Ok(resp)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_categories(&self) -> anyhow::Result<CurseForgeResponse<Vec<Category>>> {
        let mut url = self.base_url.join("categories")?;
        url.set_query(Some("gameId=432"));

        trace!("GET {}", url);

        let resp = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .json_with_context_reporting::<CurseForgeResponse<Vec<Category>>>(
                "curseforge::get_categories",
            )
            .await?;

        Ok(resp)
    }

    #[tracing::instrument(skip(self))]
    pub async fn search(
        &self,
        search_params: ModSearchParameters,
    ) -> anyhow::Result<CurseForgeResponse<Vec<Mod>>> {
        let more_than_10_categories = search_params
            .query
            .category_ids
            .as_ref()
            .map(|ids| ids.len() > 10)
            .unwrap_or(false);

        if more_than_10_categories {
            bail!("Cannot search for more than 10 categories at once");
        }

        let mut url = self.base_url.join("mods/search")?;
        let query = search_params.query.into_query_parameters()?;
        url.set_query(Some(&query));

        trace!("GET {}", url);

        let resp = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .json_with_context_reporting::<CurseForgeResponse<Vec<Mod>>>("curseforge::search")
            .await?;

        Ok(resp)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_mod(
        &self,
        mod_parameters: ModParameters,
    ) -> anyhow::Result<CurseForgeResponse<Mod>> {
        let url = self
            .base_url
            .join(&format!("mods/{}", &mod_parameters.mod_id.to_string()))?;

        trace!("GET {}", url);

        let resp = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .json_with_context_reporting::<CurseForgeResponse<Mod>>("curseforge::get_mod")
            .await?;

        Ok(resp)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_mods(
        &self,
        mod_parameters: ModsParameters,
    ) -> anyhow::Result<CurseForgeResponse<Vec<Mod>>> {
        if mod_parameters.body.mod_ids.is_empty() {
            return Ok(CurseForgeResponse {
                data: Vec::new(),
                pagination: None,
            });
        }

        let url = self.base_url.join("mods")?;
        let body = serde_json::to_string(&mod_parameters.body)?;

        trace!("POST {url} - {body:?}");

        let resp = self
            .client
            .post(url.as_str())
            .body(body.to_string())
            .send()
            .await?
            .json_with_context_reporting::<CurseForgeResponse<Vec<Mod>>>("curseforge::get_mods")
            .await?;

        Ok(resp)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_fingerprints(
        &self,
        hashes: &[u32],
    ) -> anyhow::Result<CurseForgeResponse<FingerprintsMatchesResult>> {
        let url = self.base_url.join("fingerprints")?;
        let body = json!({ "fingerprints": hashes });

        trace!("POST {url} - {body:?}");

        let resp = self
            .client
            .post(url.as_str())
            .body(body.to_string())
            .send()
            .await?
            .json_with_context_reporting::<CurseForgeResponse<FingerprintsMatchesResult>>(
                "curseforge::get_fingerprints",
            )
            .await?;

        Ok(resp)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_mod_description(
        &self,
        mod_parameters: ModDescriptionParameters,
    ) -> anyhow::Result<CurseForgeResponse<String>> {
        let url = self.base_url.join(&format!(
            "mods/{}/description",
            &mod_parameters.mod_id.to_string()
        ))?;

        trace!("GET {}", url);

        let resp = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .json_with_context_reporting::<CurseForgeResponse<String>>(
                "curseforge::get_mod_description",
            )
            .await?;
        Ok(resp)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_mod_file(
        &self,
        mod_parameters: ModFileParameters,
    ) -> anyhow::Result<CurseForgeResponse<File>> {
        let url = self.base_url.join(&format!(
            "mods/{}/files/{}",
            &mod_parameters.mod_id.to_string(),
            &mod_parameters.file_id.to_string()
        ))?;

        trace!("GET {}", url);

        let resp = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .json_with_context_reporting::<CurseForgeResponse<File>>("curseforge::get_mod_file")
            .await?;
        Ok(resp)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_mod_files(
        &self,
        mod_parameters: ModFilesParameters,
    ) -> anyhow::Result<CurseForgeResponse<Vec<File>>> {
        let mut url = self.base_url.join(&format!(
            "mods/{}/files",
            &mod_parameters.mod_id.to_string()
        ))?;

        let query = mod_parameters.query.into_query_parameters()?;
        url.set_query(Some(&query));

        trace!("GET {}", url);

        let resp = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .json_with_context_reporting::<CurseForgeResponse<Vec<File>>>(
                "curseforge::get_mod_files",
            )
            .await?;
        Ok(resp)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_files(
        &self,
        mod_parameters: FilesParameters,
    ) -> anyhow::Result<CurseForgeResponse<Vec<File>>> {
        let url = self.base_url.join("mods/files")?;

        let body = serde_json::to_string(&mod_parameters.body)?;

        trace!("POST {url} - {body:?}");

        let resp = self
            .client
            .post(url.as_str())
            .body(reqwest::Body::from(body))
            .send()
            .await?
            .json_with_context_reporting::<CurseForgeResponse<Vec<File>>>("curseforge::get_files")
            .await?;

        Ok(resp)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_mod_file_changelog(
        &self,
        mod_parameters: ModFileChangelogParameters,
    ) -> anyhow::Result<CurseForgeResponse<String>> {
        let url = self.base_url.join(&format!(
            "mods/{}/files/{}/changelog",
            &mod_parameters.mod_id.to_string(),
            &mod_parameters.file_id.to_string()
        ))?;

        trace!("GET {}", url);

        let resp = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .json_with_context_reporting::<CurseForgeResponse<String>>(
                "curseforge::get_mod_file_changelog",
            )
            .await?;
        Ok(resp)
    }
}

// Converts a CurseForge version (manifest.json version for example) to a standard version
pub async fn convert_cf_version_to_standard_version(
    app: Arc<AppInner>,
    curseforge_version: domain::modplatforms::curseforge::manifest::Minecraft,
    dummy_string: String,
) -> anyhow::Result<StandardVersion> {
    let modloaders = curseforge_version
        .mod_loaders
        .into_iter()
        .map(|mod_loader| {
            let app = Arc::clone(&app);
            let mc_version = curseforge_version.version.clone();
            let dummy_string = dummy_string.clone();
            async move {
                let (loader, version) = mod_loader.id.split_once('-').ok_or_else(|| {
                    anyhow::anyhow!(
                        "modloader id '{}' could not be split into a name-version pair",
                        mod_loader.id
                    )
                })?;

                match loader {
                    "forge" => {
                        let forge_manifest = app.minecraft_manager().get_forge_manifest().await?;

                        let forge_version = forge_manifest
                            .game_versions
                            .into_iter()
                            .find(|v| v.id == mc_version)
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "forge manifest does not contain version '{}'",
                                    mc_version
                                )
                            })?
                            .loaders
                            .into_iter()
                            .find(|l| l.id.contains(version))
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "forge manifest does not contain loader '{}'",
                                    version
                                )
                            })?;

                        Ok(ModLoader {
                            type_: ModLoaderType::Forge,
                            version: forge_version.id.to_string(),
                        })
                    }
                    "neoforge" => {
                        let neoforge_manifest =
                            app.minecraft_manager().get_neoforge_manifest().await?;

                        let neoforge_version = neoforge_manifest
                            .game_versions
                            .into_iter()
                            .find(|v| v.id == mc_version)
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "neoforge manifest does not contain version '{}'",
                                    mc_version
                                )
                            })?
                            .loaders
                            .into_iter()
                            .find(|l| l.id.contains(version))
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "neoforge manifest does not contain loader '{}'",
                                    version
                                )
                            })?;

                        Ok(ModLoader {
                            type_: ModLoaderType::Neoforge,
                            version: neoforge_version.id.to_string(),
                        })
                    }
                    "fabric" => {
                        let fabric_manifest = app.minecraft_manager().get_fabric_manifest().await?;

                        let fabric_version = fabric_manifest
                            .game_versions
                            .into_iter()
                            .find(|v| v.id == dummy_string)
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "fabric manifest does not contain version '{}'",
                                    mc_version
                                )
                            })?
                            .loaders
                            .into_iter()
                            .find(|l| l.id.contains(version))
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "fabric manifest does not contain loader '{}'",
                                    version
                                )
                            })?;

                        Ok(ModLoader {
                            type_: ModLoaderType::Fabric,
                            version: fabric_version.id.to_string(),
                        })
                    }
                    "quilt" => {
                        let quilt_manifest = app.minecraft_manager().get_quilt_manifest().await?;

                        let quilt_version = quilt_manifest
                            .game_versions
                            .into_iter()
                            .find(|v| v.id == mc_version)
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "quilt manifest does not contain version '{}'",
                                    mc_version
                                )
                            })?
                            .loaders
                            .into_iter()
                            .find(|l| l.id.contains(version))
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "quilt manifest does not contain loader '{}'",
                                    version
                                )
                            })?;

                        Ok(ModLoader {
                            type_: ModLoaderType::Quilt,
                            version: quilt_version.id.to_string(),
                        })
                    }
                    _ => bail!("unsupported modloader '{loader}'"),
                }
            }
        });

    let modloaders = futures::future::try_join_all(modloaders)
        .await?
        .into_iter()
        .collect();

    let gdl_version = StandardVersion {
        release: curseforge_version.version.clone(),
        modloaders,
    };

    Ok(gdl_version)
}

pub fn convert_standard_version_to_cf_version(
    standard_version: StandardVersion,
) -> anyhow::Result<domain::modplatforms::curseforge::manifest::Minecraft> {
    let mod_loaders: Result<
        Vec<domain::modplatforms::curseforge::manifest::ModLoaders>,
        anyhow::Error,
    > = standard_version
        .modloaders
        .into_iter()
        .enumerate()
        .map(|(i, loader)| {
            let id = match loader.type_ {
                ModLoaderType::Forge => {
                    let split = loader.version.split('-').nth(1).ok_or(anyhow::anyhow!(
                        "forge version '{}' could not be split into a name-version pair",
                        loader.version
                    ))?;

                    Ok::<_, anyhow::Error>(format!("forge-{}", split))
                }
                ModLoaderType::Neoforge => Ok(format!("neoforge-{}", loader.version)),
                ModLoaderType::Fabric => Ok(format!("fabric-{}", loader.version)),
                ModLoaderType::Quilt => Ok(format!("quilt-{}", loader.version)),
            }?;

            Ok(domain::modplatforms::curseforge::manifest::ModLoaders {
                id,
                primary: i == 0,
            })
        })
        .collect();

    let mod_loaders = mod_loaders?;

    let cf_version = domain::modplatforms::curseforge::manifest::Minecraft {
        version: standard_version.release,
        mod_loaders,
    };

    Ok(cf_version)
}

#[cfg(test)]
mod test {
    use crate::domain::modplatforms::curseforge::filters::{
        ModFilesParametersQuery, ModSearchParametersQuery,
    };

    #[tokio::test]
    async fn test_search_no_query() {
        use super::*;

        let client = crate::iridium_client::get_client().build();
        let curseforge = CurseForge::new(client);

        let search_params = ModSearchParameters {
            query: ModSearchParametersQuery {
                game_id: 432,
                category_ids: None,
                game_version: None,
                index: None,
                page_size: None,
                search_filter: None,
                slug: None,
                class_id: None,
                game_version_type_id: None,
                mod_loader_types: None,
                author_id: None,
                sort_field: None,
                sort_order: None,
            },
        };

        let mods = curseforge.search(search_params).await.unwrap();
        assert!(!mods.data.is_empty());
    }

    #[tokio::test]
    async fn test_search_with_query() {
        use super::*;

        let client = crate::iridium_client::get_client().build();
        let curseforge = CurseForge::new(client);

        let search_params = ModSearchParameters {
            query: ModSearchParametersQuery {
                game_id: 432,
                category_ids: None,
                game_version: None,
                index: None,
                page_size: None,
                search_filter: Some("jei".to_string()),
                slug: None,
                class_id: None,
                game_version_type_id: None,
                mod_loader_types: None,
                author_id: None,
                sort_field: None,
                sort_order: None,
            },
        };

        let mods = curseforge.search(search_params).await.unwrap();
        assert!(!mods.data.is_empty());
    }

    #[tokio::test]
    async fn test_get_mod() {
        use super::*;

        let client = crate::iridium_client::get_client().build();
        let curseforge = CurseForge::new(client);

        let mod_id = 389615;

        let mod_ = curseforge.get_mod(ModParameters { mod_id }).await.unwrap();
        assert_eq!(mod_.data.id, mod_id);
    }

    #[tokio::test]
    async fn test_get_mod_description() {
        use super::*;

        let client = crate::iridium_client::get_client().build();
        let curseforge = CurseForge::new(client);

        let mod_id = 389615;

        let mod_ = curseforge
            .get_mod_description(ModDescriptionParameters { mod_id })
            .await
            .unwrap();
        assert_ne!(mod_.data.len(), 0);
    }

    #[tokio::test]
    async fn test_get_mod_file() {
        use super::*;

        let client = crate::iridium_client::get_client().build();
        let curseforge = CurseForge::new(client);

        let mod_id = 389615;
        let file_id = 3931045;

        let mod_ = curseforge
            .get_mod_file(ModFileParameters { mod_id, file_id })
            .await
            .unwrap();
        assert_eq!(mod_.data.id, file_id);
    }

    #[tokio::test]
    async fn test_get_mod_files() {
        use super::*;

        let client = crate::iridium_client::get_client().build();
        let curseforge = CurseForge::new(client);

        let mod_id = 389615;

        let mod_ = curseforge
            .get_mod_files(ModFilesParameters {
                mod_id,
                query: ModFilesParametersQuery {
                    game_version: None,
                    index: None,
                    page_size: None,
                    game_version_type_id: None,
                    mod_loader_type: None,
                },
            })
            .await
            .unwrap();
        assert!(!mod_.data.is_empty());
    }

    #[tokio::test]
    async fn test_get_mod_file_changelog() {
        use super::*;

        let client = crate::iridium_client::get_client().build();
        let curseforge = CurseForge::new(client);

        let mod_id = 389615;
        let file_id = 3931045;

        let mod_ = curseforge
            .get_mod_file_changelog(ModFileChangelogParameters { mod_id, file_id })
            .await
            .unwrap();
        assert_ne!(mod_.data.len(), 0);
    }
}
