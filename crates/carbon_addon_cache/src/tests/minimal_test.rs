use crate::coordinator::CacheCoordinator;
use crate::storage::StorageConfig;
use crate::tests::mock_storage::MockAddonStorage;
use std::sync::Arc;
use tempfile::TempDir;

#[tokio::test]
async fn test_minimal_coordinator_creation() {
    println!("=== MINIMAL TEST START ===");

    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig {
        runtime_path: temp_dir.path().to_path_buf(),
        max_image_size: 1024 * 1024,
        max_cache_size: 100 * 1024 * 1024,
        cleanup_interval: 3600,
    };

    let storage = Arc::new(MockAddonStorage::new());

    println!("Creating coordinator...");
    let coordinator = CacheCoordinator::new(storage, config);

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

    println!("=== MINIMAL TEST END ===");
}

#[tokio::test]
async fn test_minimal_coordinator_start_stop() {
    println!("=== MINIMAL START/STOP TEST START ===");

    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig {
        runtime_path: temp_dir.path().to_path_buf(),
        max_image_size: 1024 * 1024,
        max_cache_size: 100 * 1024 * 1024,
        cleanup_interval: 3600,
    };

    let storage = Arc::new(MockAddonStorage::new());
    let mut coordinator = CacheCoordinator::new(storage, config).unwrap();

    println!("Starting coordinator with timeout...");
    let start_result =
        tokio::time::timeout(std::time::Duration::from_secs(5), coordinator.start()).await;

    match start_result {
        Ok(Ok(())) => {
            println!("Coordinator started successfully");

            println!("Shutting down coordinator...");
            let shutdown_result =
                tokio::time::timeout(std::time::Duration::from_secs(3), coordinator.shutdown())
                    .await;

            match shutdown_result {
                Ok(Ok(())) => println!("Coordinator shut down successfully"),
                Ok(Err(e)) => println!("Shutdown error: {}", e),
                Err(_) => println!("Shutdown timed out"),
            }
        }
        Ok(Err(e)) => {
            println!("Start error: {}", e);
        }
        Err(_) => {
            println!("Start timed out after 5 seconds");
        }
    }

    println!("=== MINIMAL START/STOP TEST END ===");
}
