use std::collections::HashMap;
use std::path::Path;

use futures::Future;
use md5::Md5;
use sha2::Digest;
use sha2::Sha512;

use crate::util::NormalizedWalkdir;

pub async fn scan_dir(path: &Path, filter: Option<&Vec<&str>>) -> anyhow::Result<super::PackInfo> {
    let mut futures = Vec::new();

    let mut walker = NormalizedWalkdir::new(path)?;
    while let Some(entry) = walker.next()? {
        if entry.is_dir {
            continue;
        }

        let path = entry.entry.path();
        let mut relpath = entry.relative_path.to_string();

        if let Some(filter) = filter.as_ref() {
            if !filter.contains(&(&relpath as &str)) {
                continue;
            }
        }

        futures.push(async move {
            if relpath.ends_with(".disabled") {
                relpath.truncate(relpath.len() - ".disabled".len());
            }

            let mut file = tokio::fs::File::open(path).await?;
            let mut sha512 = Sha512::new();
            let mut md5 = Md5::new();

            carbon_scheduler::buffered_digest(&mut file, |chunk| {
                sha512.update(&chunk);
                md5.update(&chunk);
            }).await?;

            let sha512 = sha512.finalize().into();
            let md5 = md5.finalize().into();

            Ok::<_, anyhow::Error>((relpath, super::FileHashes { sha512, md5 }))
        });
    }

    let hashes = futures::future::join_all(futures)
        .await
        .into_iter()
        .collect::<Result<HashMap<_, _>, _>>()?;

    Ok(super::PackInfo { files: hashes })
}
