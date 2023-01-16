use std::{
    io::{Read, Seek},
    path::{Path, PathBuf},
};
use thiserror::Error;
use tracing::trace;

#[derive(Error, Debug)]
pub enum CompressionError {
    #[error("Failed to open file: {0}")]
    IOError(#[from] std::io::Error),
    #[error("The provided file is not a supported compression format")]
    UnknownFormat,
    #[error("Failed during decompression task execution: {0}")]
    GenericDecompressionError(#[from] tokio::task::JoinError),
    #[error("Failed to process zip file: {0}")]
    ZipError(#[from] zip::result::ZipError),
}

#[derive(Debug, PartialEq)]
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
            [.., 0x75, 0x73, 0x74, 0x61, 0x72] => Ok(Self::Tar),
            _ => Err(CompressionError::UnknownFormat),
        }
    }
}

fn detect_compression_format<T>(file: &mut T) -> Result<CompressionFormat, CompressionError>
where
    T: Read,
{
    let mut header = [0; 262]; // Magic number to hold the max possible offset (tar) of 257 bytes + 5 relevant bytes
    let _bytes = file.read(&mut header)?;
    if let Ok(format) = CompressionFormat::from_bytes(&header) {
        return Ok(format);
    }

    Err(CompressionError::UnknownFormat)
}

fn decompress_zip<R>(
    archive: &mut zip::ZipArchive<R>,
    dest: PathBuf,
) -> Result<(), CompressionError>
where
    R: Read + Seek,
{
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
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
            std::fs::create_dir_all(&outpath)?;
        } else {
            trace!(
                "File {} extracted to \"{}\" ({} bytes)",
                i,
                outpath.display(),
                file.size()
            );
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    std::fs::create_dir_all(p)?;
                }
            }
            let mut outfile = std::fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }

        // Get and Set permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            if let Some(mode) = file.unix_mode() {
                std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode))?;
            }
        }
    }

    Ok(())
}

fn decompress_tar<R>(
    archive: &mut tar::Archive<R>,
    dest_folder: PathBuf,
) -> Result<(), CompressionError>
where
    R: Read,
{
    archive.unpack(dest_folder)?;
    Ok(())
}

fn decompress_gzip(archive_path: PathBuf, dest_folder: PathBuf) -> Result<(), CompressionError> {
    let archive_file = std::fs::File::open(&archive_path)?;
    let mut archive = flate2::read::GzDecoder::new(archive_file);

    let format = detect_compression_format(&mut archive);
    // Redeclare because it's been consumed by detect_compression_format
    let archive_file = std::fs::File::open(archive_path)?;
    let archive = flate2::read::GzDecoder::new(archive_file);

    if let Ok(format) = format {
        if CompressionFormat::Tar == format {
            let mut archive = tar::Archive::new(archive);
            decompress_tar(&mut archive, dest_folder)?;
            return Ok(());
        } else {
            unimplemented!("{format:?} is not supported yet")
        }
    }

    unimplemented!("Gzip is not supported yet");
}

// TODO: Ideally decompression would recursively look for another compressed file until it can't find any more.

// accept both paths and strings
pub async fn decompress<T>(path: T, dest_folder: &Path) -> Result<(), CompressionError>
where
    T: AsRef<Path> + Send + Sync,
{
    let path_clone = path.as_ref().to_path_buf();
    let dest_folder_clone = dest_folder.to_path_buf();

    let task_handler = tokio::task::spawn_blocking(move || {
        let mut file = std::fs::File::open(&path_clone)?;
        let file_clone = std::fs::File::open(&path_clone)?;
        let format = detect_compression_format(&mut file)?;
        trace!("Starting compression for file: {path_clone:?} to {dest_folder_clone:?}");
        trace!("Detected compression format: {format:?}");

        match format {
            CompressionFormat::Zip => {
                let mut archive = zip::ZipArchive::new(file_clone)?;
                decompress_zip(&mut archive, dest_folder_clone)?;
            }
            CompressionFormat::Tar => {
                let mut archive = tar::Archive::new(file_clone);
                decompress_tar(&mut archive, dest_folder_clone)?;
            }
            CompressionFormat::Gzip => {
                decompress_gzip(path_clone, dest_folder_clone)?;
            }
        }

        Ok::<(), CompressionError>(())
    });

    task_handler.await??;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_detect_compression_format() {
        let mut file = std::fs::File::open("fixtures/compressed.zip").unwrap();
        let format = detect_compression_format(&mut file).unwrap();
        assert_eq!(CompressionFormat::Zip, format);

        let mut file = std::fs::File::open("fixtures/compressed.tar").unwrap();
        let format = detect_compression_format(&mut file).unwrap();
        assert_eq!(CompressionFormat::Tar, format);

        let mut file = std::fs::File::open("fixtures/compressed.tar.gz").unwrap();
        let format = detect_compression_format(&mut file).unwrap();
        assert_eq!(CompressionFormat::Gzip, format);

        let mut file = std::fs::File::open("fixtures/compressed.gz").unwrap();
        let format = detect_compression_format(&mut file).unwrap();
        assert_eq!(CompressionFormat::Gzip, format);
    }

    #[tokio::test]
    async fn test_decompress_zip() {
        let file_path = PathBuf::from("fixtures/compressed.zip");
        let dest_folder = PathBuf::from("tests_decompressed/zip");
        decompress(&file_path, &dest_folder).await.unwrap();
    }

    #[tokio::test]
    async fn test_decompress_tar() {
        let file_path = PathBuf::from("fixtures/compressed.tar");
        let dest_folder = PathBuf::from("tests_decompressed/tar");
        decompress(&file_path, &dest_folder).await.unwrap();
    }

    #[tokio::test]
    async fn test_decompress_tar_gzip() {
        let file_path = PathBuf::from("fixtures/compressed.tar.gz");
        let dest_folder = PathBuf::from("tests_decompressed/tar_gzip");
        decompress(&file_path, &dest_folder).await.unwrap();
    }

    // Not supported yet.. :(

    // #[tokio::test]
    // async fn test_decompress_gzip() {
    //     let file_path = PathBuf::from("fixtures/compressed.gz");
    //     let dest_folder = PathBuf::from("tests_decompressed/gzip");
    //     decompress(&file_path, &dest_folder).await.unwrap();
    // }
}
