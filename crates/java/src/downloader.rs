use anyhow::{bail, Ok, Result};
use async_zip::read::seek::ZipFileReader;
use futures::StreamExt;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::fs::{create_dir_all, File, OpenOptions};

pub enum JavaMajorSemVer {
    _8,
    _17,
}

impl<'a> From<JavaMajorSemVer> for &'a str {
    fn from(version: JavaMajorSemVer) -> &'a str {
        match version {
            JavaMajorSemVer::_8 => "8",
            JavaMajorSemVer::_17 => "17",
        }
    }
}

fn get_download_url(version: &str) -> String {
    let java_os = match std::env::consts::OS {
        "linux" => "linux",
        "windows" => "windows",
        "macos" => "mac",
        _ => panic!("Unsupported OS"),
    };

    let java_arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "x86" => "x86",
        "aarch64" => "aarch64",
        _ => panic!("Unsupported architecture"),
    };

    format!(
        "https://api.adoptopenjdk.net/v3/assets/latest/{}/hotspot?architecture={}&image_type=jre&jvm_impl=hotspot&os={}&page=0&page_size=1&project=jdk&sort_method=DEFAULT&sort_order=DESC",
        version, java_arch, java_os
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

    let assets = reqwest::get(url).await?.json::<Vec<Asset>>().await?;
    let asset = assets
        .first()
        .ok_or_else(|| anyhow::anyhow!("Can't find a java asset"))?;

    // Download to disk
    let mut resp_stream = reqwest::get(&asset.binary.package.link)
        .await?
        .bytes_stream();
    let runtime = base_path.join("runtime");
    tokio::fs::create_dir_all(&runtime).await?;

    let mut file = File::create(runtime.join(format!("{}.zip", asset.release_name))).await?;

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

    // Unzip
    let mut zip = ZipFileReader::new(&mut file);

    Ok(())
}

fn sanitize_file_path(path: &str) -> PathBuf {
    // Replaces backwards slashes
    path.replace('\\', "/")
        // Sanitizes each component
        .split('/')
        .map(sanitize_filename::sanitize)
        .collect()
}

/// Extracts everything from the ZIP archive to the output directory
async fn unzip_file(archive: File, out_dir: &Path) {
    let mut reader = ZipFileReader::new(archive)
        .await
        .expect("Failed to read ZipFile");
    for index in 0..reader.file().entries().len() {
        let entry = &reader.file().entries().get(index).unwrap().entry();
        let path = out_dir.join(sanitize_file_path(entry.filename()));
        // If the filename of the entry ends with '/', it is treated as a directory.
        // This is implemented by previous versions of this crate and the Python Standard Library.
        // https://docs.rs/async_zip/0.0.8/src/async_zip/read/mod.rs.html#63-65
        // https://github.com/python/cpython/blob/820ef62833bd2d84a141adedd9a05998595d6b6d/Lib/zipfile.py#L528
        let entry_is_dir = entry.filename().ends_with('/');

        let mut entry_reader = reader.entry(index).await.expect("Failed to read ZipEntry");

        if entry_is_dir {
            // The directory may have been created if iteration is out of order.
            if !path.exists() {
                create_dir_all(&path)
                    .await
                    .expect("Failed to create extracted directory");
            }
        } else {
            // Creates parent directories. They may not exist if iteration is out of order
            // or the archive does not contain directory entries.
            let parent = path
                .parent()
                .expect("A file entry should have parent directories");
            if !parent.is_dir() {
                create_dir_all(parent)
                    .await
                    .expect("Failed to create parent directories");
            }
            let mut writer = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .await
                .expect("Failed to create extracted file");
            tokio::io::copy(&mut entry_reader, &mut writer)
                .await
                .expect("Failed to copy to extracted file");

            // Closes the file and manipulates its metadata here if you wish to preserve its metadata from the archive.
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_download_url() {
        let current_path = std::env::current_dir().unwrap();

        setup_jre(current_path, JavaMajorSemVer::_8).await.unwrap();
    }
}
