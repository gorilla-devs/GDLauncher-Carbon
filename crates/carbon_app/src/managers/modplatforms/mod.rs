use super::ManagerRef;
use crate::domain::modplatforms::curseforge as cf;
use crate::{iridium_client::get_client, managers::GDL_API_BASE};

mod curseforge;

pub struct ModplatformsManager {
    pub curseforge: curseforge::CurseForge,
}

impl ModplatformsManager {
    pub fn new() -> Self {
        Self {
            curseforge: curseforge::CurseForge::new(get_client()),
        }
    }
}

impl ManagerRef<'_, ModplatformsManager> {}

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
