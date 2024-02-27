use std::{collections::HashSet, path::PathBuf, sync::Arc};

use anyhow::Context;
use daedalus::minecraft::{AssetIndex, AssetsIndex};
use prisma_client_rust::QueryError;
use thiserror::Error;
use tokio::sync::Mutex;
use tracing::trace;

use crate::{db::PrismaClient, domain::runtime_path::AssetsPath};

#[derive(Error, Debug)]
pub enum AssetsError {
    #[error("Can't fetch assets index manifest: {0}")]
    FetchAssetsIndexManifest(#[from] reqwest::Error),
    #[error("Can't execute db query: {0}")]
    QueryError(#[from] QueryError),
}

pub async fn get_meta(
    db_client: Arc<PrismaClient>,
    reqwest_client: reqwest_middleware::ClientWithMiddleware,
    version_asset_index: &AssetIndex,
    asset_indexes_path: PathBuf,
) -> anyhow::Result<(AssetsIndex, Vec<u8>)> {
    static LOCK: Mutex<()> = Mutex::const_new(());
    let _guard = LOCK.lock().await;

    let db_cache = db_client
        .assets_meta_cache()
        .find_unique(crate::db::assets_meta_cache::id::equals(
            version_asset_index.id.clone(),
        ))
        .exec()
        .await?;

    if let Some(db_cache) = db_cache {
        let asset_index = serde_json::from_slice(&db_cache.assets_index);

        if let Ok(asset_index) = asset_index {
            trace!("Asset index {} found in cache", version_asset_index.id);
            return Ok((asset_index, db_cache.assets_index));
        } else {
            tracing::warn!(
                "Failed to deserialize asset index for {} from cache, re-fetching: {}",
                version_asset_index.id,
                db_cache.id
            );
        }
    }

    let asset_index = reqwest_client
        .get(version_asset_index.url.clone())
        .send()
        .await?
        .bytes()
        .await?;

    db_client
        .assets_meta_cache()
        .upsert(
            crate::db::assets_meta_cache::id::equals(version_asset_index.id.clone()),
            crate::db::assets_meta_cache::create(
                version_asset_index.id.clone(),
                asset_index.to_vec(),
                vec![],
            ),
            vec![crate::db::assets_meta_cache::assets_index::set(
                asset_index.to_vec(),
            )],
        )
        .exec()
        .await?;

    Ok((serde_json::from_slice(&asset_index)?, asset_index.to_vec()))
}

pub enum AssetsDir {
    Index(PathBuf),
    Virtual(PathBuf),
    InstanceMapped(PathBuf),
}

impl AssetsDir {
    pub fn to_path_buf(&self) -> PathBuf {
        match self {
            Self::Index(buf) => buf.clone(),
            Self::Virtual(buf) => buf.clone(),
            Self::InstanceMapped(buf) => buf.clone(),
        }
    }
}

pub async fn get_assets_dir(
    db_client: Arc<PrismaClient>,
    reqwest_client: reqwest_middleware::ClientWithMiddleware,
    version_assets_index: &AssetIndex,
    assets_path: AssetsPath,
    resources_dir: PathBuf,
) -> anyhow::Result<AssetsDir> {
    let (assets_index, _) = get_meta(
        db_client,
        reqwest_client,
        version_assets_index,
        assets_path.get_indexes_path(),
    )
    .await?;

    if assets_index.map_virtual {
        Ok(AssetsDir::Virtual(
            assets_path
                .get_virtual_path()
                .join(version_assets_index.id.clone()),
        ))
    } else if assets_index.map_to_resources {
        Ok(AssetsDir::InstanceMapped(resources_dir))
    } else {
        Ok(AssetsDir::Index(assets_path.to_path()))
    }
}

pub async fn reconstruct_assets(
    db_client: Arc<PrismaClient>,
    reqwest_client: reqwest_middleware::ClientWithMiddleware,
    version_asset_index: &AssetIndex,
    assets_path: AssetsPath,
    resources_dir: PathBuf,
) -> anyhow::Result<()> {
    let (assets_index, assets_index_bytes) = get_meta(
        db_client,
        reqwest_client,
        version_asset_index,
        assets_path.get_indexes_path(),
    )
    .await?;

    let asset_index_full_path = assets_path
        .get_indexes_path()
        .join(format!("{}.json", version_asset_index.id));

    let existing_file_size = asset_index_full_path
        .metadata()
        .map(|m| m.len())
        .unwrap_or_default();

    let expected_file_size = version_asset_index.size as u64;

    let file_ok = asset_index_full_path.exists() && existing_file_size == expected_file_size;

    if !file_ok {
        tokio::fs::create_dir_all(&assets_path.get_indexes_path()).await?;
        tokio::fs::write(asset_index_full_path, assets_index_bytes).await?;
    }

    let target_path = if assets_index.map_virtual {
        Some(
            assets_path
                .get_virtual_path()
                .join(version_asset_index.id.clone()),
        )
    } else if assets_index.map_to_resources {
        Some(resources_dir)
    } else {
        None
    };

    if let Some(target_path) = target_path {
        tokio::fs::create_dir_all(&target_path)
            .await
            .with_context(|| {
                format!(
                    "Failed to create directory `{}`",
                    target_path.to_string_lossy()
                )
            })?;

        let mut existing_files: HashSet<PathBuf> = walkdir::WalkDir::new(&target_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .map(|e| e.path().to_path_buf())
            .collect();

        let objects_path = assets_path.get_objects_path();

        for (path, object) in assets_index.objects.iter() {
            let object_path = objects_path.join(&object.hash[0..2]).join(&object.hash);
            let asset_path = target_path.join(path);

            existing_files.remove(&asset_path);

            let exists = tokio::fs::try_exists(&asset_path).await.ok();
            if exists != Some(true) {
                if let Some(parent) = asset_path.parent() {
                    tokio::fs::create_dir_all(parent).await.with_context(|| {
                        format!("Failed to create directory `{}`", parent.to_string_lossy())
                    })?;
                }
                tokio::fs::copy(&object_path, &asset_path)
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to copy file from `{}` to `{}`",
                            object_path.to_string_lossy(),
                            asset_path.to_string_lossy()
                        )
                    })?;
            }
        }

        if assets_index.map_virtual {
            for path in existing_files.iter() {
                if let Err(err) = tokio::fs::remove_file(path).await {
                    tracing::warn!(
                        "Failed to remove leftover virtual asset `{}` during reconstruction: {}",
                        path.to_string_lossy(),
                        err
                    );
                }
            }
        }
    }

    Ok(())
}
