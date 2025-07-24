use tempfile::TempDir;

#[tokio::test]
async fn test_just_config_creation() {
    use crate::storage::StorageConfig;

    println!("=== ULTRA MINIMAL TEST START ===");

    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig {
        runtime_path: temp_dir.path().to_path_buf(),
        max_image_size: 1024 * 1024,
        max_cache_size: 100 * 1024 * 1024,
        cleanup_interval: 3600,
    };

    println!("Created storage config: {:?}", config.runtime_path);

    drop(config);
    drop(temp_dir);

    println!("=== ULTRA MINIMAL TEST END ===");
}

#[tokio::test]
async fn test_just_file_cache_stage_creation() {
    use crate::stages::FileCache;
    use crate::storage::StorageConfig;

    println!("=== FILE CACHE ONLY TEST START ===");

    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig {
        runtime_path: temp_dir.path().to_path_buf(),
        max_image_size: 1024 * 1024,
        max_cache_size: 100 * 1024 * 1024,
        cleanup_interval: 3600,
    };

    println!("Creating just file cache stage...");

    let file_cache = FileCache::new(config);

    match file_cache {
        Ok(fc) => {
            println!("File cache created successfully!");
            drop(fc);
        }
        Err(e) => {
            println!("Failed to create file cache: {}", e);
        }
    }

    println!("=== FILE CACHE ONLY TEST END ===");
}

#[tokio::test]
async fn test_just_metadata_stage_creation() {
    use crate::stages::MetadataExtractor;
    use crate::storage::StorageConfig;

    println!("=== METADATA EXTRACTOR ONLY TEST START ===");

    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig {
        runtime_path: temp_dir.path().to_path_buf(),
        max_image_size: 1024 * 1024,
        max_cache_size: 100 * 1024 * 1024,
        cleanup_interval: 3600,
    };

    println!("Creating just metadata extractor stage...");

    let metadata = MetadataExtractor::new(config);

    match metadata {
        Ok(me) => {
            println!("Metadata extractor created successfully!");
            drop(me);
        }
        Err(e) => {
            println!("Failed to create metadata extractor: {}", e);
        }
    }

    println!("=== METADATA EXTRACTOR ONLY TEST END ===");
}

#[tokio::test]
async fn test_just_image_stage_creation() {
    use crate::stages::ImageCache;
    use crate::storage::StorageConfig;

    println!("=== IMAGE CACHE ONLY TEST START ===");

    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig {
        runtime_path: temp_dir.path().to_path_buf(),
        max_image_size: 1024 * 1024,
        max_cache_size: 100 * 1024 * 1024,
        cleanup_interval: 3600,
    };

    println!("Creating just image cache stage...");

    let image_cache = ImageCache::new(config);

    match image_cache {
        Ok(ic) => {
            println!("Image cache created successfully!");
            drop(ic);
        }
        Err(e) => {
            println!("Failed to create image cache: {}", e);
        }
    }

    println!("=== IMAGE CACHE ONLY TEST END ===");
}
