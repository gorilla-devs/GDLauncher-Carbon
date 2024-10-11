use crate::{cache_middleware, iridium_client::get_client};

use super::{ManagerRef, UnsafeAppRef};

pub mod curseforge;
pub mod modrinth;

pub struct ModplatformsManager {
    pub curseforge: curseforge::CurseForge,
    pub modrinth: modrinth::Modrinth,
}

impl ModplatformsManager {
    pub fn new(unsafeappref: UnsafeAppRef, gdl_base_api: String) -> Self {
        Self {
            curseforge: curseforge::CurseForge::new(cache_middleware::new_client(
                unsafeappref.clone(),
                get_client(gdl_base_api.clone()),
            )),
            modrinth: modrinth::Modrinth::new(cache_middleware::new_client(
                unsafeappref,
                get_client(gdl_base_api),
            )),
        }
    }
}

impl ManagerRef<'_, ModplatformsManager> {}
