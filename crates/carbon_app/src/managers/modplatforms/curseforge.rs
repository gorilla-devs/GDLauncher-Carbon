use reqwest_middleware::ClientWithMiddleware;
use serde_json::json;
use tracing::trace;
use url::Url;

use crate::{
    domain::modplatforms::curseforge::{
        filters::{
            FilesParameters, ModDescriptionParameters, ModFileChangelogParameters,
            ModFileParameters, ModFilesParameters, ModParameters, ModSearchParameters,
            ModsParameters,
        },
        Category, CurseForgeResponse, File, FingerprintsMatchesResult, Mod,
    },
    managers::GDL_API_BASE,
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
    pub async fn get_categories(&self) -> anyhow::Result<CurseForgeResponse<Vec<Category>>> {
        let mut url = self.base_url.join("categories")?;
        url.set_query(Some("gameId=432"));

        trace!("GET {}", url);

        let resp = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .json::<CurseForgeResponse<Vec<Category>>>()
            .await?;
        Ok(resp)
    }

    #[tracing::instrument(skip(self))]
    pub async fn search(
        &self,
        search_params: ModSearchParameters,
    ) -> anyhow::Result<CurseForgeResponse<Vec<Mod>>> {
        let mut url = self.base_url.join("mods/search")?;
        let query = search_params.query.into_query_parameters()?;
        url.set_query(Some(&query));

        trace!("GET {}", url);

        let resp = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .json::<CurseForgeResponse<Vec<Mod>>>()
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
            .json::<CurseForgeResponse<Mod>>()
            .await?;
        Ok(resp)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_mods(
        &self,
        mod_parameters: ModsParameters,
    ) -> anyhow::Result<CurseForgeResponse<Vec<Mod>>> {
        let url = self.base_url.join("mods")?;

        let body = serde_json::to_string(&mod_parameters.body)?;

        trace!("POST {url} - {body:?}");

        let resp = self
            .client
            .post(url.as_str())
            .body(body.to_string())
            .send()
            .await?
            .json::<CurseForgeResponse<Vec<Mod>>>()
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
            .json::<CurseForgeResponse<FingerprintsMatchesResult>>()
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
            .json::<CurseForgeResponse<String>>()
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
            .json::<CurseForgeResponse<File>>()
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
            .json::<CurseForgeResponse<Vec<File>>>()
            .await?;
        Ok(resp)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_files(
        &self,
        mod_parameters: FilesParameters,
    ) -> anyhow::Result<CurseForgeResponse<Vec<File>>> {
        let url = self.base_url.join("files")?;

        let body = serde_json::to_string(&mod_parameters.body)?;

        trace!("POST {url} - {body:?}");

        let resp = self
            .client
            .post(url.as_str())
            .json(&body)
            .send()
            .await?
            .json::<CurseForgeResponse<Vec<File>>>()
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
            .json::<CurseForgeResponse<String>>()
            .await?;
        Ok(resp)
    }
}

// #[cfg(test)]
// mod test {
//     use crate::domain::modplatforms::curseforge::filters::{
//         ModFilesParametersQuery, ModSearchParametersQuery,
//     };

//     #[tokio::test]
//     async fn test_search_no_query() {
//         use super::*;

//         let client = reqwest::Client::builder().build().unwrap();
//         let client = reqwest_middleware::ClientBuilder::new(client).build();
//         let curseforge = CurseForge::new(client);

//         let search_params = ModSearchParameters {
//             query: ModSearchParametersQuery {
//                 game_id: 432,
//                 category_id: None,
//                 game_version: None,
//                 index: None,
//                 page_size: None,
//                 search_filter: None,
//                 slug: None,
//                 class_id: None,
//                 game_version_type_id: None,
//                 mod_loader_type: None,
//                 author_id: None,
//                 sort_field: None,
//                 sort_order: None,
//             },
//         };

//         let mods = curseforge.search(search_params).await.unwrap();
//         assert!(!mods.data.is_empty());
//     }

//     #[tokio::test]
//     async fn test_search_with_query() {
//         use super::*;

//         let client = reqwest::Client::builder().build().unwrap();
//         let client = reqwest_middleware::ClientBuilder::new(client).build();
//         let curseforge = CurseForge::new(client);

//         let search_params = ModSearchParameters {
//             query: ModSearchParametersQuery {
//                 game_id: 432,
//                 category_id: None,
//                 game_version: None,
//                 index: None,
//                 page_size: None,
//                 search_filter: Some("jei".to_string()),
//                 slug: None,
//                 class_id: None,
//                 game_version_type_id: None,
//                 mod_loader_type: None,
//                 author_id: None,
//                 sort_field: None,
//                 sort_order: None,
//             },
//         };

//         let mods = curseforge.search(search_params).await.unwrap();
//         assert!(!mods.data.is_empty());
//     }

//     #[tokio::test]
//     async fn test_get_mod() {
//         use super::*;

//         let client = reqwest::Client::builder().build().unwrap();
//         let client = reqwest_middleware::ClientBuilder::new(client).build();
//         let curseforge = CurseForge::new(client);

//         let mod_id = 389615;

//         let mod_ = curseforge.get_mod(ModParameters { mod_id }).await.unwrap();
//         assert_eq!(mod_.data.id, mod_id);
//     }

//     #[tokio::test]
//     async fn test_get_mod_description() {
//         use super::*;

//         let client = reqwest::Client::builder().build().unwrap();
//         let client = reqwest_middleware::ClientBuilder::new(client).build();
//         let curseforge = CurseForge::new(client);

//         let mod_id = 389615;

//         let mod_ = curseforge
//             .get_mod_description(ModDescriptionParameters { mod_id })
//             .await
//             .unwrap();
//         assert_ne!(mod_.data.len(), 0);
//     }

//     #[tokio::test]
//     async fn test_get_mod_file() {
//         use super::*;

//         let client = reqwest::Client::builder().build().unwrap();
//         let client = reqwest_middleware::ClientBuilder::new(client).build();
//         let curseforge = CurseForge::new(client);

//         let mod_id = 389615;
//         let file_id = 3931045;

//         let mod_ = curseforge
//             .get_mod_file(ModFileParameters { mod_id, file_id })
//             .await
//             .unwrap();
//         assert_eq!(mod_.data.id, file_id);
//     }

//     #[tokio::test]
//     async fn test_get_mod_files() {
//         use super::*;

//         let client = reqwest::Client::builder().build().unwrap();
//         let client = reqwest_middleware::ClientBuilder::new(client).build();
//         let curseforge = CurseForge::new(client);

//         let mod_id = 389615;

//         let mod_ = curseforge
//             .get_mod_files(ModFilesParameters {
//                 mod_id,
//                 query: ModFilesParametersQuery {
//                     game_version: None,
//                     index: None,
//                     page_size: None,
//                     game_version_type_id: None,
//                     mod_loader_type: None,
//                 },
//             })
//             .await
//             .unwrap();
//         assert!(!mod_.data.is_empty());
//     }

//     #[tokio::test]
//     async fn test_get_mod_file_changelog() {
//         use super::*;

//         let client = reqwest::Client::builder().build().unwrap();
//         let client = reqwest_middleware::ClientBuilder::new(client).build();
//         let curseforge = CurseForge::new(client);

//         let mod_id = 389615;
//         let file_id = 3931045;

//         let mod_ = curseforge
//             .get_mod_file_changelog(ModFileChangelogParameters { mod_id, file_id })
//             .await
//             .unwrap();
//         assert_ne!(mod_.data.len(), 0);
//     }
// }
