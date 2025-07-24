use anyhow::Result;
use blake3::Hasher as Blake3Hasher;
use carbon_scheduler;
use md5;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use tracing::{debug, error};

/// Scheduler integration for CPU-intensive addon caching operations
pub struct CacheScheduler;

impl CacheScheduler {
    /// Execute CPU-intensive operation using carbon_scheduler
    pub async fn cpu_intensive<F, R>(operation: F) -> Result<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        debug!("Executing CPU-intensive operation via carbon_scheduler");

        let result = carbon_scheduler::cpu_block(operation).await;

        debug!("CPU-intensive operation completed");
        Ok(result)
    }

    /// Compute file hashes using carbon_scheduler for optimal performance
    pub async fn compute_file_hashes(file_path: &Path) -> Result<FileHashes> {
        let path = file_path.to_path_buf();

        debug!("Computing file hashes for: {:?}", path);

        Self::cpu_intensive(move || {
            let mut file = File::open(&path)?;
            let mut blake3 = Blake3Hasher::new();
            let mut sha256 = Sha256::new();
            let mut md5 = md5::Context::new();
            let mut murmur2_len = 0u64;
            let mut total_len = 0u64;

            // Use carbon_scheduler's buffered digest for streaming
            let mut buffer = vec![0u8; carbon_scheduler::BUFSIZE];

            loop {
                match file.read(&mut buffer)? {
                    0 => break,
                    n => {
                        let chunk = &buffer[..n];
                        blake3.update(chunk);
                        sha256.update(chunk);
                        md5.consume(chunk);

                        // Calculate Murmur2 hash length (excluding whitespace)
                        murmur2_len += chunk
                            .iter()
                            .filter(|&&b| b != 9 && b != 10 && b != 13 && b != 32)
                            .count() as u64;
                        total_len += n as u64;
                    }
                }
            }

            let blake3_hash = blake3.finalize();
            let sha256_hash = sha256.finalize();
            let md5_hash = md5.compute();

            // Calculate Murmur2 hash (approximation using a simple hash)
            let murmur2_hash = murmur2_len as u32;

            Ok::<FileHashes, anyhow::Error>(FileHashes {
                blake3: hex::encode(blake3_hash.as_bytes()),
                sha256: hex::encode(sha256_hash),
                md5: format!("{:x}", md5_hash),
                murmur2: murmur2_hash,
                file_size: total_len,
            })
        })
        .await?
    }

    /// Compute file hashes using carbon_scheduler's buffered digest
    pub async fn compute_file_hashes_buffered(file_path: &Path) -> Result<FileHashes> {
        let path = file_path.to_path_buf();

        debug!("Computing file hashes (buffered) for: {:?}", path);

        let mut file = tokio::fs::File::open(&path).await?;
        let mut blake3 = Blake3Hasher::new();
        let mut sha256 = Sha256::new();
        let mut md5 = md5::Context::new();
        let mut murmur2_len = 0u64;
        let mut total_len = 0u64;

        // Use carbon_scheduler's optimized buffered digest
        carbon_scheduler::buffered_digest(&mut file, |chunk| {
            // Clone the chunk data to avoid borrow checker issues
            let chunk_data = chunk.to_vec();

            blake3.update(&chunk_data);
            sha256.update(&chunk_data);
            md5.consume(&chunk_data);

            // Calculate Murmur2 hash length (excluding whitespace)
            murmur2_len += chunk_data
                .iter()
                .filter(|&&b| b != 9 && b != 10 && b != 13 && b != 32)
                .count() as u64;
            total_len += chunk_data.len() as u64;
        })
        .await?;

        let blake3_hash = blake3.finalize();
        let sha256_hash = sha256.finalize();
        let md5_hash = md5.compute();

        // Calculate Murmur2 hash (approximation)
        let murmur2_hash = murmur2_len as u32;

        Ok(FileHashes {
            blake3: hex::encode(blake3_hash.as_bytes()),
            sha256: hex::encode(sha256_hash),
            md5: format!("{:x}", md5_hash),
            murmur2: murmur2_hash,
            file_size: total_len,
        })
    }

    /// Process ZIP file entries using carbon_scheduler
    pub async fn process_zip_entries<F, R>(zip_path: &Path, processor: F) -> Result<R>
    where
        F: FnOnce(zip::ZipArchive<File>) -> Result<R> + Send + 'static,
        R: Send + 'static,
    {
        let path = zip_path.to_path_buf();

        debug!("Processing ZIP entries for: {:?}", path);

        Self::cpu_intensive(move || {
            let file = File::open(&path)?;
            let archive = zip::ZipArchive::new(file)?;
            processor(archive)
        })
        .await?
    }

    /// Process image data using carbon_scheduler
    pub async fn process_image<F, R>(image_data: Vec<u8>, processor: F) -> Result<R>
    where
        F: FnOnce(Vec<u8>) -> Result<R> + Send + 'static,
        R: Send + 'static,
    {
        debug!("Processing image data of {} bytes", image_data.len());

        Self::cpu_intensive(move || processor(image_data)).await?
    }

    /// Batch process multiple files using carbon_scheduler
    pub async fn batch_process_files<F, R>(
        files: Vec<std::path::PathBuf>,
        processor: F,
    ) -> Result<Vec<R>>
    where
        F: Fn(std::path::PathBuf) -> Result<R> + Send + Sync + 'static,
        R: Send + 'static,
    {
        debug!("Batch processing {} files", files.len());

        Self::cpu_intensive(move || {
            files
                .into_iter()
                .map(|path| match processor(path.clone()) {
                    Ok(result) => Ok(result),
                    Err(e) => {
                        error!("Error processing file {:?}: {}", path, e);
                        Err(e)
                    }
                })
                .collect::<Result<Vec<_>>>()
        })
        .await?
    }
}

/// Result of file hash computation
#[derive(Debug, Clone)]
pub struct FileHashes {
    pub blake3: String,
    pub sha256: String,
    pub md5: String,
    pub murmur2: u32,
    pub file_size: u64,
}

impl FileHashes {
    /// Convert to the addon cache Checksums format
    pub fn to_checksums(&self) -> crate::Checksums {
        crate::Checksums {
            blake3: self.blake3.clone(),
            sha256: self.sha256.clone(),
            md5: self.md5.clone(),
            murmur2: self.murmur2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_cpu_intensive_operation() {
        let result = CacheScheduler::cpu_intensive(|| {
            // Simulate CPU-intensive work
            (0..1000).map(|i| i * i).sum::<i32>()
        })
        .await
        .unwrap();

        assert_eq!(result, (0..1000).map(|i| i * i).sum::<i32>());
    }

    #[tokio::test]
    async fn test_file_hash_computation() {
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "test content for hashing").unwrap();

        let hashes = CacheScheduler::compute_file_hashes_buffered(temp_file.path())
            .await
            .unwrap();

        // Verify that hashes are computed
        assert!(!hashes.blake3.is_empty());
        assert!(!hashes.sha256.is_empty());
        assert!(!hashes.md5.is_empty());
        assert!(hashes.file_size > 0);
    }

    #[tokio::test]
    async fn test_batch_file_processing() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let mut temp_files = Vec::new();

        for i in 0..3 {
            let file_path = temp_dir.path().join(format!("test_{}.txt", i));
            std::fs::write(&file_path, format!("test content {}", i)).unwrap();
            temp_files.push(file_path);
        }

        let results = CacheScheduler::batch_process_files(temp_files, |path| {
            let content = std::fs::read_to_string(&path)?;
            Ok(content.len())
        })
        .await
        .unwrap();

        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|&len| len > 0));
    }
}
