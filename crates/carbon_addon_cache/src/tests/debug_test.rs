use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::RwLock as TokioRwLock;

#[tokio::test]
async fn test_stage_creation_individual() {
    use crate::stages::*;
    use crate::storage::StorageConfig;

    println!("=== STAGE CREATION TEST START ===");

    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig {
        runtime_path: temp_dir.path().to_path_buf(),
        max_image_size: 1024 * 1024,
        max_cache_size: 100 * 1024 * 1024,
        cleanup_interval: 3600,
    };

    println!("Creating file cache stage...");
    let file_cache = match FileCache::new(config.clone()) {
        Ok(fc) => {
            println!("File cache created successfully");
            Arc::new(TokioRwLock::new(fc))
        }
        Err(e) => {
            println!("Failed to create file cache: {}", e);
            return;
        }
    };

    println!("Creating metadata extractor stage...");
    let metadata_extractor = match MetadataExtractor::new(config.clone()) {
        Ok(me) => {
            println!("Metadata extractor created successfully");
            Arc::new(TokioRwLock::new(me))
        }
        Err(e) => {
            println!("Failed to create metadata extractor: {}", e);
            return;
        }
    };

    println!("Creating image cache stage...");
    let image_cache = match ImageCache::new(config.clone()) {
        Ok(ic) => {
            println!("Image cache created successfully");
            Arc::new(TokioRwLock::new(ic))
        }
        Err(e) => {
            println!("Failed to create image cache: {}", e);
            return;
        }
    };

    println!("Creating modplatform fetcher stage...");
    let modplatform_fetcher = match ModplatformFetcher::new(config.clone()) {
        Ok(mf) => {
            println!("Modplatform fetcher created successfully");
            Arc::new(TokioRwLock::new(mf))
        }
        Err(e) => {
            println!("Failed to create modplatform fetcher: {}", e);
            return;
        }
    };

    println!("Creating update checker stage...");
    let update_checker = match UpdateChecker::new(config.clone()) {
        Ok(uc) => {
            println!("Update checker created successfully");
            Arc::new(TokioRwLock::new(uc))
        }
        Err(e) => {
            println!("Failed to create update checker: {}", e);
            return;
        }
    };

    println!("All stages created successfully!");

    drop(file_cache);
    drop(metadata_extractor);
    drop(image_cache);
    drop(modplatform_fetcher);
    drop(update_checker);

    println!("=== STAGE CREATION TEST END ===");
}

#[tokio::test]
async fn test_coordinator_new_step_by_step() {
    use crate::coordinator::CacheCoordinator;
    use crate::storage::StorageConfig;
    use crate::tests::mock_storage::MockAddonStorage;

    println!("=== STEP BY STEP COORDINATOR TEST START ===");

    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig {
        runtime_path: temp_dir.path().to_path_buf(),
        max_image_size: 1024 * 1024,
        max_cache_size: 100 * 1024 * 1024,
        cleanup_interval: 3600,
    };

    let storage = Arc::new(MockAddonStorage::new());

    println!("About to call CacheCoordinator::new()...");

    // This is where it likely hangs
    let coordinator = CacheCoordinator::new(storage, config);

    println!("CacheCoordinator::new() completed");

    match coordinator {
        Ok(coord) => {
            println!("Coordinator created successfully");
            drop(coord);
            println!("Coordinator dropped");
        }
        Err(e) => {
            println!("Failed to create coordinator: {}", e);
        }
    }

    println!("=== STEP BY STEP COORDINATOR TEST END ===");
}
