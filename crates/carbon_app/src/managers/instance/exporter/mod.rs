use std::{collections::HashMap, path::PathBuf, sync::Arc};

use anyhow::Context;
use async_zip::{tokio::write::ZipFileWriter, Compression, ZipDateTime, ZipEntryBuilder};

use glob::Pattern as GlobPattern;
use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;
use tokio::io::AsyncReadExt;

use crate::{
    domain::{instance::InstanceId, vtask::VisualTaskId},
    managers::{minecraft::absolute_clean_path, AppInner},
};

pub mod curseforge;
pub mod modrinth;

#[derive(Debug, Serialize, Deserialize, EnumIter)]
pub enum ExportFormat {
    CurseForgeZip,
    MRPack,
}

impl ExportFormat {
    pub fn get_available() -> Vec<Self> {
        use strum::IntoEnumIterator;
        Self::iter().collect()
    }
}

#[async_trait::async_trait]
pub trait InstanceExporter {
    async fn export(
        &self,
        app: Arc<AppInner>,
        instance_id: InstanceId,
        output_path: PathBuf,
    ) -> anyhow::Result<VisualTaskId>;
}

pub struct ArchiveExporter {
    output_path: PathBuf,
    input_dir: PathBuf,
    files: Vec<PathBuf>,
    follow_symlinks: bool,
    exclude_patterns: Vec<GlobPattern>,
    extra_files: HashMap<String, Vec<u8>>,
    directory_prefix: String,
}

impl ArchiveExporter {
    pub fn new(
        output_path: PathBuf,
        input_dir: PathBuf,
        files: Vec<PathBuf>,
        directory_prefix: String,
        follow_symlinks: bool,
    ) -> Self {
        Self {
            output_path,
            input_dir,
            files,
            follow_symlinks,
            exclude_patterns: Vec::new(),
            extra_files: HashMap::new(),
            directory_prefix,
        }
    }

    pub fn add_exclude_patterns<'a, I>(&mut self, patterns: I) -> anyhow::Result<()>
    where
        I: IntoIterator<Item = &'a str>,
    {
        self.exclude_patterns.extend(
            patterns
                .into_iter()
                .map(GlobPattern::new)
                .collect::<Result<Vec<GlobPattern>, _>>()?,
        );
        Ok(())
    }

    pub fn add_extra_file(&mut self, file_name: String, data: Vec<u8>) -> bool {
        self.extra_files.insert(file_name, data).is_some()
    }

    pub async fn export(&self) -> anyhow::Result<()> {
        if tokio::fs::try_exists(&self.input_dir).await? {
            return Err(anyhow::anyhow!("Input directory does not exist"));
        }

        let input_dir = absolute_clean_path(&self.input_dir)?;

        let mut out_file = tokio::fs::File::create(&self.output_path)
            .await
            .with_context(|| {
                format!(
                    "Error opening file {} for writing",
                    &self.output_path.to_string_lossy()
                )
            })?;
        let mut archive = ZipFileWriter::with_tokio(&mut out_file);

        for (file_name, data) in &self.extra_files {
            let entry = ZipEntryBuilder::new(file_name.clone().into(), Compression::Deflate);
            archive.write_entry_whole(entry, data).await?;
        }

        for input_file in &self.files {
            let mut absolute_path = absolute_clean_path(input_file)?;
            let relative_path = absolute_path
                .strip_prefix(&input_dir)
                .with_context(|| {
                    format!(
                        "Input file {} is not under {}",
                        &absolute_path.to_string_lossy(),
                        &input_dir.to_string_lossy()
                    )
                })?
                .to_path_buf();
            if self.follow_symlinks {
                absolute_path = tokio::fs::canonicalize(absolute_path).await?
            }

            if !(self
                .exclude_patterns
                .iter()
                .any(|pattern| pattern.matches_path(&relative_path)))
            {
                let mut file = tokio::fs::File::open(&absolute_path).await?;
                let metadata = file.metadata().await?;
                let mut entry = ZipEntryBuilder::new(
                    relative_path.to_string_lossy().to_string().into(),
                    Compression::Deflate,
                );
                if let Ok(last_modified) = metadata.modified() {
                    let last_modified: chrono::DateTime<chrono::Utc> = last_modified.into();
                    entry = entry.last_modification_date(ZipDateTime::from_chrono(&last_modified));
                }
                let mut data = vec![];
                file.read_to_end(&mut data).await?;
                archive.write_entry_whole(entry, &data).await?;
            }
        }

        archive.close().await?;

        Ok(())
    }
}
