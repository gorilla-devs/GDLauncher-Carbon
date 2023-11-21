//! This module handles importing Curseforge instance imports.

use super::{ImportScanStatus, InstanceImporter};
use crate::{domain::vtask::VisualTaskId, managers::AppInner};
use std::{path, sync::Arc};

/// Curseforge instance importer.
#[derive(Debug)]
pub struct CurseforgeInstanceImporter {}

#[async_trait]
impl InstanceImporter for CurseforgeInstanceImporter {
    async fn scan(&self, app: &Arc<AppInner>, scan_path: path::PathBuf) -> anyhow::Result<()> {
        todo!()
    }

    async fn get_status(&self) -> ImportScanStatus {
        todo!()
    }

    async fn begin_import(
        &self,
        app: &Arc<AppInner>,
        index: u32,
        name: Option<String>,
    ) -> anyhow::Result<VisualTaskId> {
        todo!()
    }
}
