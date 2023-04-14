use reqwest::{Certificate, Identity};

use super::ManagerRef;

pub struct ModplatformsManager {}

impl ModplatformsManager {
    pub fn new() -> Self {
        Self {}
    }
}

#[cfg(feature = "iridium_lib")]
#[inline(always)]
fn get_client() -> reqwest_middleware::ClientWithMiddleware {
    iridium::get_client()
}

#[cfg(not(feature = "iridium_lib"))]
#[inline(always)]
fn get_client() -> reqwest_middleware::ClientWithMiddleware {
    let client = reqwest::Client::builder().build().unwrap();
    reqwest_middleware::ClientBuilder::new(client).build()
}

impl ManagerRef<'_, ModplatformsManager> {
    pub async fn some_api_request(&self) -> anyhow::Result<()> {
        let client = get_client();
        let response = client
            .get("https://api.gdlauncher.com/v1/curseforge/mods/520914")
            .send()
            .await?;

        println!("{:?}", response);

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::setup_managers_for_test;

    use super::*;

    #[tokio::test]
    async fn test_get_client() {
        let client = get_client();

        let response = client
            // .get("https://api.gdlauncher.com/v1/curseforge/mods/520914")
            .get("https://api.gdlauncher.com/cf/mods/520914")
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
    }
}
