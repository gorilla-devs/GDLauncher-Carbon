use crate::error::JavaError;
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
    download: Vec<Downloadable>,
}

#[async_trait::async_trait]
trait JavaAuto {
    async fn setup(
        &self,
        base_path: &Path,
        progress_report: Sender<JavaProgress>,
    ) -> Result<(), JavaError>;
    async fn get_runtime_assets(&self, runtime_path: &Path) -> Result<JavaMeta, JavaError>;
    fn locate_binary(&self, base_path: &Path) -> PathBuf;
    async fn check_for_updates(&self, runtime_path: &Path) -> Result<bool, JavaError>;
    async fn update(&mut self) -> Result<(), JavaError>;
}
