use carbon_net::Downloadable;
use chrono::{DateTime, FixedOffset};
use std::path::{Path, PathBuf};
use tokio::sync::watch::Sender;

mod adoptopenjdk;
mod mojang;

#[derive(Default)]
pub struct JavaProgress {
    pub current: u64,
    pub total: u64,
    pub step: String,
}

struct JavaMeta {
    last_updated: DateTime<FixedOffset>,
    extract_folder_name: String,
    download: Vec<Downloadable>,
}

#[async_trait::async_trait]
trait JavaAuto {
    async fn setup(
        &mut self,
        base_path: &Path,
        progress_report: Sender<JavaProgress>,
    ) -> anyhow::Result<()>;
    async fn get_runtime_assets(&self, runtime_path: &Path) -> anyhow::Result<JavaMeta>;
    fn locate_binary(&self, base_path: &Path) -> anyhow::Result<PathBuf>;
    async fn check_for_updates(&self, runtime_path: &Path) -> anyhow::Result<bool>;
    async fn update(&mut self) -> anyhow::Result<()>;
}
