use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
use thiserror::Error;
use tracing::trace;

#[derive(Error, Debug)]
pub enum AssetsError {
    #[error("failed to download asset index")]
    DownloadError(#[from] reqwest::Error),
    #[error("failed to parse asset index")]
    ParseError(#[from] serde_json::Error),
    #[error("failed to write asset index")]
    WriteError(#[from] std::io::Error),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AssetIndex {
    pub objects: HashMap<String, Object>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Object {
    pub hash: String,
    pub size: i64,
}

impl AssetIndex {
    #[tracing::instrument]
    pub async fn get_asset_downloads(
        &self,
        base_path: &PathBuf,
    ) -> Result<Vec<carbon_net::Downloadable>, AssetsError> {
        trace!("Downloading assets");

        let mut files: Vec<carbon_net::Downloadable> = vec![];

        for (_, object) in self.objects.iter() {
            // TODO: handle directories for different versions (virtual legacy)
            let asset_path = base_path
                .join("assets")
                .join("objects")
                .join(&object.hash[0..2])
                .join(&object.hash);

            files.push(
                carbon_net::Downloadable::new(
                    format!(
                        "https://resources.download.minecraft.net/{}/{}",
                        &object.hash[0..2],
                        &object.hash
                    ),
                    asset_path,
                )
                .with_checksum(Some(carbon_net::Checksum::Sha1(object.hash.clone())))
                .with_size(object.size as u64),
            );
        }

        Ok(files)
    }
}
