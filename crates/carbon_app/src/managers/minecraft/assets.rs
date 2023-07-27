use std::{collections::HashSet, path::PathBuf};

use anyhow::Context;
use daedalus::minecraft::{AssetIndex, AssetsIndex};
use prisma_client_rust::QueryError;
use thiserror::Error;

use crate::domain::runtime_path::AssetsPath;

#[derive(Error, Debug)]
pub enum AssetsError {
    #[error("Can't fetch assets index manifest: {0}")]
    FetchAssetsIndexManifest(#[from] reqwest::Error),
    #[error("Can't execute db query: {0}")]
    QueryError(#[from] QueryError),
}

pub async fn get_meta(
    reqwest_client: reqwest_middleware::ClientWithMiddleware,
    version_asset_index: AssetIndex,
    asset_indexes_path: PathBuf,
) -> anyhow::Result<AssetsIndex> {
    let asset_index_bytes = reqwest_client
        .get(version_asset_index.url)
        .send()
        .await?
        .bytes()
        .await?;

    tokio::fs::create_dir_all(&asset_indexes_path).await?;
    tokio::fs::write(
        asset_indexes_path.join(format!("{}.json", version_asset_index.id)),
        asset_index_bytes.clone(),
    )
    .await?;

    Ok(serde_json::from_slice(&asset_index_bytes)?)
}

pub async fn load_index(
    index_id: &str,
    asset_indexes_path: PathBuf,
) -> anyhow::Result<AssetsIndex> {
    let asset_index_bytes = tokio::fs::read(asset_indexes_path.join(format!("{}.json", index_id)))
        .await
        .with_context(|| {
            format!(
                "Failed to read asset index `{}` from `{}`",
                index_id,
                asset_indexes_path.to_string_lossy()
            )
        })?;
    Ok(serde_json::from_slice(&asset_index_bytes)?)
}

pub async fn get_assets_dir(
    index_id: &str,
    assets_path: AssetsPath,
    resources_dir: PathBuf,
) -> anyhow::Result<PathBuf> {
    let assets_index = load_index(index_id, assets_path.get_indexes_path()).await?;
    if assets_index.map_virtual {
        Ok(assets_path.get_virtual_path().join(index_id))
    } else if assets_index.map_to_resources {
        Ok(resources_dir)
    } else {
        Ok(assets_path.to_path())
    }
}

pub async fn reconstruct_assets(
    index_id: &str,
    assets_path: AssetsPath,
    resources_dir: PathBuf,
) -> anyhow::Result<()> {
    let assets_index = load_index(index_id, assets_path.get_indexes_path()).await?;
    let target_path = if assets_index.map_virtual {
        Some(assets_path.get_virtual_path().join(index_id))
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
