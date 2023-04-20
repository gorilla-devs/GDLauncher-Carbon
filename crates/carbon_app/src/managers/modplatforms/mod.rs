use reqwest::{Certificate, Identity};

use crate::{iridium_client::get_client, managers::GDL_API_BASE};

use super::ManagerRef;

mod curseforge;

pub struct ModplatformsManager {}

impl ModplatformsManager {
    pub fn new() -> Self {
        Self {}
    }
}

impl ManagerRef<'_, ModplatformsManager> {
    pub async fn some_api_request(&self) -> anyhow::Result<()> {
        let client = get_client();
        let response = client
            .get(format!("{}/v1/curseforge/mods/520914", GDL_API_BASE))
            .send()
            .await?;

        println!("{:?}", response);

        Ok(())
    }
}

// #[cfg(test)]
// mod test {
//     use crate::setup_managers_for_test;

//     use super::*;

//     #[tokio::test]
//     async fn test_get_client() {
//         let client = get_client();

//         let response = client
//             // .get("https://api.gdlauncher.com/v1/curseforge/mods/520914")
//             .get("https://api.gdlauncher.com/cf/mods/520914")
//             .send()
//             .await
//             .unwrap();

//         assert_eq!(response.status(), 200);
//     }
// }
