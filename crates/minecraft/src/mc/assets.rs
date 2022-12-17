use anyhow::{bail, Context, Ok, Result};
use futures::StreamExt;
use reqwest::Client;
use reqwest_middleware::ClientBuilder;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::{borrow::Borrow, collections::HashMap, path::PathBuf, sync::Arc};
use tracing::{debug, trace};

use crate::net::download_multiple;

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
    pub async fn download_assets(&self, assets_path: PathBuf) -> Result<()> {
        trace!("Downloading assets");

        let mut files: Vec<crate::net::Download> = vec![];

        for (path, object) in self.objects.iter() {
            let file_path = assets_path.join(path);

            files.push(crate::net::Download {
                url: format!(
                    "https://resources.download.minecraft.net/{}/{}",
                    &object.hash[0..2],
                    &object.hash
                ),
                path: file_path,
                checksum: Some(crate::net::Checksum::Sha1(object.hash.clone())),
                size: Some(object.size as u64),
            });
        }

        let (progress, mut receiver) = tokio::sync::watch::channel(0);
        let mut last_progress = 0;
        let download_handle = tokio::spawn(async move {
            download_multiple(files, progress).await?;
            Ok(())
        });

        while (receiver.changed().await).is_ok() {
            if *receiver.borrow() != last_progress {
                last_progress = *receiver.borrow();
                trace!("Downloaded {} - {}", *receiver.borrow(), self.objects.len());
            }
        }

        download_handle.await??;

        Ok(())
    }
}
