use reqwest::{Certificate, Identity};

use super::ManagerRef;

pub struct ModplatformsManager {}

impl ModplatformsManager {
    pub fn new() -> Self {
        Self {}
    }
}

#[cfg(feature = "iridium_lib")]
fn get_client() -> reqwest::Client {
    let cert = iridium::retrieve_certificate();

    let identity = Identity::from_pem(&cert).unwrap();
    let certificate = Certificate::from_pem(&cert).unwrap();

    reqwest::Client::builder()
        .tls_built_in_root_certs(false)
        .use_rustls_tls()
        .identity(identity)
        .add_root_certificate(certificate)
        .https_only(true)
        .build()
        .unwrap()
}

#[cfg(not(feature = "iridium_lib"))]
fn get_client() -> reqwest::Client {
    reqwest::Client::new()
}

impl ManagerRef<'_, ModplatformsManager> {
    //
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
