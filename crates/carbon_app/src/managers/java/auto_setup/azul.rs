use std::{path::PathBuf, sync::Arc};

use tokio::sync::watch::Sender;

use crate::db::PrismaClient;

use super::{AutoSetup, JavaProgress};

pub struct Azul;

#[async_trait::async_trait]
impl AutoSetup for Azul {
    async fn setup(
        &mut self,
        base_path: PathBuf,
        db_client: &Arc<PrismaClient>,
        progress_report: Sender<JavaProgress>,
    ) -> anyhow::Result<()> {
        todo!()
    }
}
