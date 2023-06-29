use std::path::Path;

mod legacy_gdlauncher;

pub enum Entity {
    LegacyGDLauncher,
}

#[async_trait::async_trait]
pub trait InstanceImporter {
    async fn scan(&self, path: &Path) -> anyhow::Result<()>;
    async fn import(&self, path: &Path) -> anyhow::Result<()>;
}

pub fn get_importer(entity: Entity) -> impl InstanceImporter {
    match entity {
        Entity::LegacyGDLauncher => legacy_gdlauncher::LegacyGDLauncherImporter,
    }
}
