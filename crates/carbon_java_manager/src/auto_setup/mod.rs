use std::path::{Path, PathBuf};

use tokio::sync::watch::Sender;
use tracing::trace;

use crate::{constants::JAVA_RUNTIMES_FOLDER, error::JavaError};

mod adoptopenjdk;
mod mojang;

#[derive(Default)]
pub struct JavaProgress {
    pub current: u64,
    pub total: u64,
    pub step: String,
}

#[async_trait::async_trait]
trait JavaAuto {
    async fn setup(
        &self,
        base_path: &Path,
        progress_report: Sender<JavaProgress>,
    ) -> Result<(), JavaError>;
    async fn get_runtime_meta<T>(&self) -> Result<T, JavaError>
    where
        T: serde::de::DeserializeOwned + for<'de> serde::Deserialize<'de>;
    fn locate_binary(&self, base_path: &Path) -> PathBuf;
    async fn check_for_updates(&self) -> Result<(), JavaError>;
    async fn update(&mut self) -> Result<(), JavaError>;
}
