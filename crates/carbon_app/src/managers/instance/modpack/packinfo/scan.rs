use std::collections::HashMap;
use std::path::Path;

use futures::Future;
use md5::Md5;
use sha2::Digest;
use sha2::Sha512;

use crate::util::NormalizedWalkdir;

pub async fn scan_dir(path: &Path) -> anyhow::Result<super::PackInfo> {
    let mut futures = Vec::new();

    let mut walker = NormalizedWalkdir::new(path)?;
    while let Some(entry) = walker.next()? {
        if entry.is_dir {
            continue;
        }

        let path = entry.entry.path();
        let relpath = entry.relative_path.to_string();
        futures.push(async move {
            let content = tokio::fs::read(path).await?;

            let hashes = tokio::task::spawn_blocking(move || {
                let sha512: [u8; 64] = Sha512::digest(&content).into();
                let md5 = Md5::digest(&content).into();

                super::FileHashes {
                    sha512,
                    md5,
                }
            })
            .await?;

            Ok::<_, anyhow::Error>((relpath, hashes))
        });
    }

    let hashes = futures::future::join_all(futures)
        .await
        .into_iter()
        .collect::<Result<HashMap<_, _>, _>>()?;

    Ok(super::PackInfo { files: hashes })
}
