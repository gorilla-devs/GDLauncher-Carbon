use std::{path::PathBuf, sync::Arc};

use anyhow::Result;
use futures::StreamExt;
use reqwest::Client;
use reqwest_middleware::ClientBuilder;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use sha1::{Digest, Sha1};
use sha2::Sha256;
use tokio::{fs::OpenOptions, io::AsyncReadExt};

#[derive(Debug)]
pub enum Checksum {
    Sha1(String),
    Sha256(String),
}

#[derive(Debug)]
pub struct Download {
    pub url: String,
    pub path: PathBuf,
    pub checksum: Option<Checksum>,
    pub size: Option<u64>,
}

pub async fn download_multiple(
    files: Vec<Download>,
    progress: tokio::sync::watch::Sender<i32>,
) -> Result<()> {
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
    let reqwest_client = Client::builder().build().unwrap();
    let client = ClientBuilder::new(reqwest_client)
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build();

    let downloads = Arc::new(tokio::sync::Semaphore::new(10));

    let mut tasks: Vec<tokio::task::JoinHandle<Result<()>>> = vec![];

    for file in files {
        let semaphore = Arc::clone(&downloads);
        let url = file.url.clone();
        let path = file.path.clone();
        let client = client.clone();

        tasks.push(tokio::spawn(async move {
            let _permit = semaphore.acquire().await?;
            let path = path.clone();
            let url = url.clone();

            let file_looks_good = match file.size {
                Some(size) if path.exists() => {
                    let metadata = tokio::fs::metadata(&path).await;
                    if let Ok(metadata) = metadata {
                        metadata.len() == size
                    } else {
                        false
                    }
                }
                Some(_) => false,
                None => true,
            };

            // verify if file exists and checksum matches
            if file_looks_good {
                let mut sha1 = Sha1::new();
                let mut sha256 = Sha256::new();

                let mut fs_file = tokio::fs::File::open(&path).await?;

                let mut buf = vec![];
                fs_file.read_to_end(&mut buf).await?;

                match file.checksum {
                    Some(Checksum::Sha1(_)) => sha1.update(&buf),
                    Some(Checksum::Sha256(_)) => sha256.update(&buf),
                    None => {}
                }

                match file.checksum {
                    Some(Checksum::Sha1(ref hash)) => {
                        let finalized = sha1.finalize();
                        if hash == &format!("{finalized:x}") {
                            return Ok(());
                        } else {
                            println!(
                                "Hash mismatch sha1 for file: {} - expected: {hash} - got: {}",
                                path.display(),
                                &format!("{finalized:x}")
                            );
                        }
                    }
                    Some(Checksum::Sha256(ref hash)) => {
                        let finalized = sha256.finalize();
                        if hash == &format!("{finalized:x}") {
                            return Ok(());
                        } else {
                            println!(
                                "Hash mismatch sha256 for file: {} - expected: {hash} - got: {}",
                                path.display(),
                                &format!("{finalized:x}")
                            );
                        }
                    }
                    None => {}
                }
            }

            let mut resp_stream = client.get(&url).send().await?.bytes_stream();

            tokio::fs::create_dir_all(
                path.parent()
                    .ok_or_else(|| anyhow::anyhow!("Can't find parent path for asset"))?,
            )
            .await?;

            let mut sha1 = Sha1::new();
            let mut sha256 = Sha256::new();

            let mut fs_file = OpenOptions::new()
                .create(!path.exists())
                .write(true)
                .truncate(path.exists())
                .open(&path)
                .await?;

            while let Some(item) = resp_stream.next().await {
                let res = item?;
                match file.checksum {
                    Some(Checksum::Sha1(_)) => sha1.update(&res),
                    Some(Checksum::Sha256(_)) => sha256.update(&res),
                    None => {}
                }
                tokio::io::copy(&mut res.as_ref(), &mut fs_file).await?;
            }

            match file.checksum {
                Some(Checksum::Sha1(hash)) => {
                    if hash != format!("{:x}", sha1.finalize()) {
                        return Err(anyhow::anyhow!("Checksum mismatch"));
                    }
                }
                Some(Checksum::Sha256(hash)) => {
                    if hash != format!("{:x}", sha256.finalize()) {
                        return Err(anyhow::anyhow!("Checksum mismatch"));
                    }
                }
                None => {}
            }

            Ok(())
        }));
    }

    let mut count = 0;

    for task in tasks {
        task.await??;
        count += 1;
        progress.send(count)?;
    }

    Ok(())
}
