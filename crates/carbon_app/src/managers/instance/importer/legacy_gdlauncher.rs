use super::InstanceImporter;

pub struct LegacyGDLauncherImporter;

#[async_trait::async_trait]
impl InstanceImporter for LegacyGDLauncherImporter {
    async fn scan(&self, path: &std::path::Path) -> anyhow::Result<()> {
        Ok(())
    }

    async fn import(&self, path: &std::path::Path) -> anyhow::Result<()> {
        Ok(())
    }
}

struct LegacyGDLauncherConfig {
    loader: _Loader,
    time_played: u64,
    background: String,
    last_played: u64,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct _Loader {
    loader_type: String,
    mc_version: String,
    file_id: u64,
    project_id: u64,
    source: String,
    source_name: String,
}
