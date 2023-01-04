use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::io::AsyncReadExt;
use tracing::trace;

// thiserror

#[derive(Error, Debug)]
pub enum CompressionError {
    #[error("Failed to open file: {0}")]
    FileOpenError(#[from] std::io::Error),
    #[error("The provided file is not a supported compression format")]
    UnknownFormat,
    #[error("Failed during decompression task execution: {0}")]
    GenericDecompressionError(#[from] tokio::task::JoinError),
}

enum CompressionFormat {
    Zip,
    Tar,
    Gzip,
}

impl CompressionFormat {
    fn from_bytes(bytes: &[u8]) -> Result<Self, CompressionError> {
        match bytes {
            [0x50, 0x4B, 0x03, 0x04, ..] => Ok(Self::Zip),
            [0x1F, 0x8B, ..] => Ok(Self::Gzip),
            [0x75, 0x73, 0x74, 0x61, 0x72, ..] => Ok(Self::Tar),
            _ => Err(CompressionError::UnknownFormat),
        }
    }
}

async fn detect_compression_format(path: &Path) -> Result<CompressionFormat, CompressionError> {
    let file = tokio::fs::File::open(path).await?;
    let mut reader = tokio::io::BufReader::new(file);
    let mut header = [0; 5];
    reader.read_exact(&mut header).await?;

    CompressionFormat::from_bytes(&header)
}

fn decompress_zip(file: std::fs::File, dest: PathBuf) -> Result<(), CompressionError> {
    let mut archive = zip::ZipArchive::new(file).unwrap();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let outpath = match file.enclosed_name() {
            Some(path) => Path::new(&dest).join(path),
            None => continue,
        };

        {
            let comment = file.comment();
            if !comment.is_empty() {
                trace!("File {i} comment: {comment}");
            }
        }

        if (*file.name()).ends_with('/') {
            trace!("File {} extracted to \"{}\"", i, outpath.display());
            std::fs::create_dir_all(&outpath).unwrap();
        } else {
            trace!(
                "File {} extracted to \"{}\" ({} bytes)",
                i,
                outpath.display(),
                file.size()
            );
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    std::fs::create_dir_all(p).unwrap();
                }
            }
            let mut outfile = std::fs::File::create(&outpath).unwrap();
            std::io::copy(&mut file, &mut outfile).unwrap();
        }

        // Get and Set permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            if let Some(mode) = file.unix_mode() {
                std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode)).unwrap();
            }
        }
    }

    Ok(())
}

fn decompress_tar(file: std::fs::File, dest: PathBuf) -> Result<(), CompressionError> {
    let mut archive = tar::Archive::new(file);
    archive.unpack(dest)?;

    Ok(())
}

fn decompress_gzip(file: std::fs::File, dest: PathBuf) -> Result<(), CompressionError> {
    let mut archive = flate2::read::GzDecoder::new(file);
    let mut out_file = std::fs::File::create(dest)?;
    std::io::copy(&mut archive, &mut out_file)?;

    Ok(())
}

pub async fn decompress(
    path: &Path,
    dest_folder: &Path,
    deep: bool,
) -> Result<(), CompressionError> {
    let format = detect_compression_format(path).await?;
    let path_clone = path.to_path_buf();
    let dest_folder_clone = dest_folder.to_path_buf();

    let task_handler = tokio::task::spawn_blocking(move || {
        let file = std::fs::File::open(path_clone).unwrap();
        match format {
            CompressionFormat::Zip => {
                decompress_zip(file, dest_folder_clone)?;
            }
            CompressionFormat::Tar => {
                decompress_tar(file, dest_folder_clone)?;
            }
            CompressionFormat::Gzip => {
                decompress_gzip(file, dest_folder_clone)?;
            }
        }

        Ok::<(), CompressionError>(())
    });

    task_handler.await??;

    Ok(())
}
