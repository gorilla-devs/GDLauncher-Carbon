use reqwest_middleware::ClientWithMiddleware;
use url::Url;

use crate::{
    domain::modplatforms::curseforge::{
        search::ModSearchParameters, Category, CurseForgeResponse, Mod,
    },
    managers::GDL_API_BASE,
};

pub struct CurseForge {
    client: ClientWithMiddleware,
    base_url: Url,
}

impl CurseForge {
    pub fn new(client: reqwest_middleware::ClientWithMiddleware) -> Self {
        Self {
            client,
            base_url: format!("{GDL_API_BASE}/cf/").parse().unwrap(),
        }
    }

    pub async fn get_categories(&self) -> anyhow::Result<Vec<Category>> {
        let mut url = self.base_url.join("categories")?;
        url.set_query(Some("gameId=432"));

        let resp = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .json::<Vec<Category>>()
            .await?;
        Ok(resp)
    }

    pub async fn search(
        &self,
        search_params: ModSearchParameters,
    ) -> anyhow::Result<CurseForgeResponse<Vec<Mod>>> {
        let mut url = self.base_url.join("mods/search")?;
        let query = search_params.into_query_parameters()?;
        url.set_query(Some(&query));

        let resp = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .json::<CurseForgeResponse<Vec<Mod>>>()
            .await?;
        Ok(resp)
    }

    pub async fn get_mod(&self, mod_id: u32) -> anyhow::Result<CurseForgeResponse<Mod>> {
        let url = self.base_url.join("mods/")?.join(&mod_id.to_string())?;

        let resp = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .json::<CurseForgeResponse<Mod>>()
            .await?;
        Ok(resp)
    }
}

#[cfg(test)]
mod test {
    #[tokio::test]
    async fn test_search_no_query() {
        use super::*;

        let client = reqwest::Client::builder().build().unwrap();
        let client = reqwest_middleware::ClientBuilder::new(client).build();
        let curseforge = CurseForge::new(client);

        let search_params = ModSearchParameters {
            game_id: 432,
            category_id: None,
            game_version: None,
            index: None,
            page_size: None,
            search_filter: None,
            slug: None,
            class_id: None,
            game_version_type_id: None,
            page: None,
            mod_loader_type: None,
            author_id: None,
            sort_field: None,
            sort_order: None,
        };

        let mods = curseforge.search(search_params).await.unwrap();
        assert!(!mods.data.is_empty());
    }

    #[tokio::test]
    async fn test_search_with_query() {
        use super::*;

        let client = reqwest::Client::builder().build().unwrap();
        let client = reqwest_middleware::ClientBuilder::new(client).build();
        let curseforge = CurseForge::new(client);

        let search_params = ModSearchParameters {
            game_id: 432,
            category_id: None,
            game_version: None,
            index: None,
            page_size: None,
            search_filter: Some("jei".to_string()),
            slug: None,
            class_id: None,
            game_version_type_id: None,
            page: None,
            mod_loader_type: None,
            author_id: None,
            sort_field: None,
            sort_order: None,
        };

        let mods = curseforge.search(search_params).await.unwrap();
        assert!(!mods.data.is_empty());
    }
}
