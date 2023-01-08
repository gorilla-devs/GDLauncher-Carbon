use futures::{StreamExt, TryFutureExt};
use sha2::Digest;
use std::path::PathBuf;
use tokio::fs::OpenOptions;

use crate::error::JavaError;

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

// TODO: fallback to x64 if arm64 is not available (through rosetta)
async fn get_adoptopenjdk_meta(version: &str) -> Result<Vec<Asset>, JavaError> {
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

    let url = format!(
        "https://api.adoptopenjdk.net/v3/assets/latest/{version}/hotspot?architecture={java_arch}&image_type=jre&jvm_impl=hotspot&os={java_os}&page=0&page_size=1&project=jdk&sort_method=DEFAULT&sort_order=DESC",
    );

    let res = reqwest::get(url)
        .await
        .map_err(JavaError::CannotRetrieveOpenJDKAssets)?;

    res.json()
        .map_err(JavaError::CannotParseAdoptOpenJDKMeta)
        .await
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

pub async fn setup_openjdk_jre(
    base_path: PathBuf,
    version: JavaMajorSemVer,
) -> Result<(), JavaError> {
    let adoptopenjdk_meta = get_adoptopenjdk_meta(version.into()).await?;

    let asset = adoptopenjdk_meta
        .first()
        .ok_or(JavaError::NoAdoptOpenJDKMetaValidVersion)?;
    let release_name = asset.release_name.clone();

    // // Download to disk
    let mut resp_stream = reqwest::get(&asset.binary.package.link)
        .await?
        .bytes_stream();

    let runtime = base_path.join("java_runtime").join("openjdk");
    tokio::fs::create_dir_all(&runtime)
        .await
        .map_err(JavaError::CannotCreateJavaOpenJDKRuntimeDirectory)?;

    let zip_path = runtime.join(format!("{release_name}.tar.gz"));

    let mut file = OpenOptions::new()
        .write(true)
        .read(true)
        .create_new(true)
        .open(&zip_path)
        .await
        .map_err(JavaError::CannotCreateJavaOpenJDKFile)?;

    let mut hasher = sha2::Sha256::new();
    while let Some(item) = resp_stream.next().await {
        let res = item?;
        let cloned = res.clone();
        tokio::io::copy(&mut res.as_ref(), &mut file)
            .await
            .map_err(JavaError::CannotCreateJavaOpenJDKFile)?;
        hasher.update(cloned);
    }

    if format!("{:x}", hasher.finalize()) != asset.binary.package.checksum {
        return Err(JavaError::ChecksumMismatch);
    }
    carbon_compression::decompress(&zip_path, &runtime).await?;

    tokio::fs::remove_file(&zip_path)
        .await
        .map_err(JavaError::CannotDeletePreviouslyDownloadedJavaOpenJDKFile)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_setup_openjdk_jre() {
        let current_path = std::env::current_dir().unwrap();

        setup_openjdk_jre(current_path, JavaMajorSemVer::Version17)
            .await
            .unwrap();
    }
}
