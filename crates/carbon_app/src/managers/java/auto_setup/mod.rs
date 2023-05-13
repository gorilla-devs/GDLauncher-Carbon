use carbon_net::Downloadable;
use chrono::{DateTime, FixedOffset};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::watch::Sender;

use crate::db::PrismaClient;

// mod adoptopenjdk;
// mod mojang;
mod azul;

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
trait AutoSetup {
    async fn setup(
        &mut self,
        base_path: PathBuf,
        db_client: &Arc<PrismaClient>,
        progress_report: Sender<JavaProgress>,
    ) -> anyhow::Result<()>;
}
