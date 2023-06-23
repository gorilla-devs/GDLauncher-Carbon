use crate::{cache_middleware, iridium_client::get_client};

use super::{ManagerRef, UnsafeAppRef};

pub mod curseforge;
pub mod modrinth;

pub struct ModplatformsManager {
    pub curseforge: curseforge::CurseForge,
}

impl ModplatformsManager {
    pub fn new(unsafeappref: UnsafeAppRef) -> Self {
        Self {
            curseforge: curseforge::CurseForge::new(cache_middleware::new_client(
                unsafeappref,
                get_client(),
            )),
        }
    }
}

impl ManagerRef<'_, ModplatformsManager> {}
