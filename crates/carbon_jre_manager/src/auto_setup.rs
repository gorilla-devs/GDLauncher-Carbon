use anyhow::{bail, Context, Ok, Result};
use futures::StreamExt;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::fs::{create_dir_all, File, OpenOptions};
use tracing::trace;

pub enum JavaMajorSemVer {
    Version8,
    Version17,
}

impl<'a> From<JavaMajorSemVer> for &'a str {
    fn from(version: JavaMajorSemVer) -> &'a str {
        match version {
            JavaMajorSemVer::Version8 => "8",
            JavaMajorSemVer::Version17 => "17",
        }
    }
}

fn get_download_url(version: &str) -> String {
    let java_os = match std::env::consts::OS {
        "linux" => "linux",
        "windows" => "windows",
        "macos" => "mac",
        _ => unreachable!("Unsupported OS"),
    };

    let java_arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "x86" => "x86",
        "aarch64" => "aarch64",
        _ => unreachable!("Unsupported architecture"),
    };

    format!(
        "https://api.adoptopenjdk.net/v3/assets/latest/{version}/hotspot?architecture={java_arch}&image_type=jre&jvm_impl=hotspot&os={java_os}&page=0&page_size=1&project=jdk&sort_method=DEFAULT&sort_order=DESC",
    )
}

#[derive(Debug, serde::Deserialize)]
struct Asset {
    binary: Binary,
    release_name: String,
}

#[derive(Debug, serde::Deserialize)]
struct Binary {
    package: Package,
}

#[derive(Debug, serde::Deserialize)]
struct Package {
    link: String,
    checksum: String,
    size: u64,
}

pub async fn setup_jre(base_path: PathBuf, version: JavaMajorSemVer) -> Result<()> {
    let url = get_download_url(version.into());
    println!("url: {}", url);

    let assets = reqwest::get(url).await?.json::<Vec<Asset>>().await?;
    let asset = assets
        .first()
        .ok_or_else(|| anyhow::anyhow!("Can't find a java asset"))?;
    let release_name = asset.release_name.clone();

    // // Download to disk
    let mut resp_stream = reqwest::get(&asset.binary.package.link)
        .await?
        .bytes_stream();
    let runtime = base_path.join("runtime");
    tokio::fs::create_dir_all(&runtime).await?;

    let zip_path = runtime.join(format!("{release_name}.tar.gz"));

    let mut file = OpenOptions::new()
        .write(true)
        .read(true)
        .create_new(true)
        .open(&zip_path)
        .await
        .context("Failed to create extracted file")?;

    let mut hasher = sha2::Sha256::new();
    while let Some(item) = resp_stream.next().await {
        let res = item?;
        let cloned = res.clone();
        tokio::io::copy(&mut res.as_ref(), &mut file).await?;
        hasher.update(cloned);
    }

    if format!("{:x}", hasher.finalize()) != asset.binary.package.checksum {
        bail!("Java asset checksum mismatch");
    }
    carbon_compression::decompress(&zip_path, &runtime).await?;

    // tokio::fs::remove_file(cloned_zip_path).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_download_url() {
        let current_path = std::env::current_dir().unwrap();

        setup_jre(current_path, JavaMajorSemVer::Version17)
            .await
            .unwrap();
    }
}
