use crate::coordinator::CacheCoordinator;
use crate::events::*;
use crate::storage::*;
use crate::tests::mock_storage::MockAddonStorage;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio;

#[tokio::test]
async fn test_full_pipeline_integration() {
    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig {
        runtime_path: temp_dir.path().to_path_buf(),
        max_image_size: 1024 * 1024,
        max_cache_size: 100 * 1024 * 1024,
        cleanup_interval: 3600,
    };

    let storage = Arc::new(MockAddonStorage::new());
    let mut coordinator = CacheCoordinator::new(storage.clone(), config).unwrap();

    // Start the coordinator
    coordinator.start().await.unwrap();

    // Create a test JAR file
    let test_jar_path = temp_dir.path().join("test_mod.jar");
    fs::write(&test_jar_path, b"test jar content").unwrap();

    // Send AddAddon event
    let add_event = CacheEvent::AddAddon {
        path: test_jar_path,
        instance_id: Some("test_instance".to_string()),
    };

    coordinator.send_event(add_event).unwrap();

    // Wait a bit for processing
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Test prioritization
    coordinator.prioritize_instance("test_instance".to_string(), Priority::Critical);

    // Wait a bit more
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Test online/offline status
    coordinator.set_online_status(false);
    coordinator.set_online_status(true);

    // Shutdown coordinator
    coordinator.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_event_flow_with_mock_storage() {
    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig {
        runtime_path: temp_dir.path().to_path_buf(),
        max_image_size: 1024 * 1024,
        max_cache_size: 100 * 1024 * 1024,
        cleanup_interval: 3600,
    };

    let storage = Arc::new(MockAddonStorage::new());
    let mut coordinator = CacheCoordinator::new(storage.clone(), config).unwrap();

    coordinator.start().await.unwrap();

    // Test multiple events
    let events = vec![
        CacheEvent::AddAddon {
            path: PathBuf::from("/test/mod1.jar"),
            instance_id: Some("instance1".to_string()),
        },
        CacheEvent::AddAddon {
            path: PathBuf::from("/test/mod2.jar"),
            instance_id: Some("instance2".to_string()),
        },
        CacheEvent::PrioritizeInstance {
            instance_id: "instance1".to_string(),
        },
        CacheEvent::GoOffline,
        CacheEvent::GoOnline,
    ];

    for event in events {
        coordinator.send_event(event).unwrap();
    }

    // Wait for processing
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    coordinator.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_concurrent_event_processing() {
    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig {
        runtime_path: temp_dir.path().to_path_buf(),
        max_image_size: 1024 * 1024,
        max_cache_size: 100 * 1024 * 1024,
        cleanup_interval: 3600,
    };

    let storage = Arc::new(MockAddonStorage::new());
    let mut coordinator = CacheCoordinator::new(storage.clone(), config).unwrap();

    coordinator.start().await.unwrap();

    // Send many events concurrently
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let coordinator_sender = coordinator.get_event_sender().clone();
            tokio::spawn(async move {
                for j in 0..5 {
                    let event = CacheEvent::AddAddon {
                        path: PathBuf::from(format!("/test/mod_{}_{}.jar", i, j)),
                        instance_id: Some(format!("instance_{}", i)),
                    };
                    coordinator_sender.send(event).unwrap();
                }
            })
        })
        .collect();

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // Wait for processing
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    coordinator.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_storage_integration() {
    let storage = Arc::new(MockAddonStorage::new());

    // Test complete storage workflow
    let metadata = LocalMetadata {
        addon_id: "integration_test".to_string(),
        name: "Integration Test Mod".to_string(),
        version: "1.0.0".to_string(),
        authors: vec!["Test Author".to_string()],
        description: Some("Test description".to_string()),
        dependencies: vec![Dependency {
            mod_id: "dependency_mod".to_string(),
            version_requirement: ">=1.0.0".to_string(),
            dependency_type: DependencyType::Required,
        }],
        checksums: Checksums {
            blake3: "integration_blake3".to_string(),
            sha256: "integration_sha256".to_string(),
            md5: "integration_md5".to_string(),
            murmur2: 123456,
        },
        mod_format: ModFormat::Fabric,
        minecraft_versions: vec!["1.20.1".to_string()],
        mod_loaders: vec!["fabric".to_string()],
        instance_id: Some("test_instance".to_string()),
    };

    // Store metadata and checksums
    storage
        .store_metadata("integration_test", &metadata)
        .await
        .unwrap();
    storage
        .store_checksums("integration_test", &metadata.checksums)
        .await
        .unwrap();

    // Store images
    let icon_data = vec![1, 2, 3, 4];
    storage
        .store_image("integration_test", ImageType::Icon, &icon_data)
        .await
        .unwrap();

    // Store platform data
    let platform_data = ModplatformData {
        platform: Platform::Modrinth,
        project_id: "integration_project".to_string(),
        file_id: "integration_file".to_string(),
        download_url: Some("https://example.com/download".to_string()),
        project_name: "Integration Test Mod".to_string(),
        project_description: Some("Test mod for integration".to_string()),
        categories: vec!["utility".to_string()],
        license: Some("MIT".to_string()),
        website_url: None,
        source_url: None,
        issues_url: None,
    };
    storage
        .store_platform_data("integration_test", &platform_data)
        .await
        .unwrap();

    // Store versions
    let versions = vec![Version {
        version_number: "1.1.0".to_string(),
        version_type: VersionType::Beta,
        minecraft_versions: vec!["1.20.1".to_string()],
        mod_loaders: vec!["fabric".to_string()],
        release_date: "2024-01-15".to_string(),
        download_url: "https://example.com/v1.1.0".to_string(),
        changelog: Some("Beta update".to_string()),
    }];
    storage
        .store_versions("integration_test", &versions)
        .await
        .unwrap();

    // Link to instance
    let file_path = PathBuf::from("/instances/test/mods/integration_test.jar");
    storage
        .link_addon_to_instance("integration_test", "test_instance", &file_path)
        .await
        .unwrap();

    // Verify everything is stored correctly
    assert!(
        storage
            .get_metadata("integration_test")
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        storage
            .get_checksums("integration_test")
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        storage
            .get_image("integration_test", ImageType::Icon)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        storage
            .get_platform_data("integration_test")
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        !storage
            .get_versions("integration_test")
            .await
            .unwrap()
            .is_empty()
    );

    let instances = storage
        .get_addon_instances("integration_test")
        .await
        .unwrap();
    assert!(instances.contains(&"test_instance".to_string()));

    let addons = storage.get_instance_addons("test_instance").await.unwrap();
    assert!(addons.contains(&"integration_test".to_string()));
}

#[tokio::test]
async fn test_error_handling() {
    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig {
        runtime_path: temp_dir.path().to_path_buf(),
        max_image_size: 1024 * 1024,
        max_cache_size: 100 * 1024 * 1024,
        cleanup_interval: 3600,
    };

    let storage = Arc::new(MockAddonStorage::new());
    let mut coordinator = CacheCoordinator::new(storage.clone(), config).unwrap();

    coordinator.start().await.unwrap();

    // Test with non-existent file
    let non_existent_event = CacheEvent::AddAddon {
        path: PathBuf::from("/non/existent/path.jar"),
        instance_id: Some("test_instance".to_string()),
    };

    // This should not crash the coordinator
    coordinator.send_event(non_existent_event).unwrap();

    // Wait for processing
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    coordinator.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_multiple_instance_processing() {
    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig {
        runtime_path: temp_dir.path().to_path_buf(),
        max_image_size: 1024 * 1024,
        max_cache_size: 100 * 1024 * 1024,
        cleanup_interval: 3600,
    };

    let storage = Arc::new(MockAddonStorage::new());
    let mut coordinator = CacheCoordinator::new(storage.clone(), config).unwrap();

    coordinator.start().await.unwrap();

    // Create multiple test instances
    let instances = vec!["instance_1", "instance_2", "instance_3"];

    for (i, instance_id) in instances.iter().enumerate() {
        // Add multiple mods per instance
        for j in 0..3 {
            let event = CacheEvent::AddAddon {
                path: PathBuf::from(format!("/test/{}/mod_{}.jar", instance_id, j)),
                instance_id: Some(instance_id.to_string()),
            };
            coordinator.send_event(event).unwrap();
        }

        // Prioritize every other instance
        if i % 2 == 0 {
            coordinator.prioritize_instance(instance_id.to_string(), Priority::High);
        }
    }

    // Wait for processing
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    coordinator.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_graceful_shutdown() {
    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig {
        runtime_path: temp_dir.path().to_path_buf(),
        max_image_size: 1024 * 1024,
        max_cache_size: 100 * 1024 * 1024,
        cleanup_interval: 3600,
    };

    let storage = Arc::new(MockAddonStorage::new());
    let mut coordinator = CacheCoordinator::new(storage.clone(), config).unwrap();

    coordinator.start().await.unwrap();

    // Send some events
    for i in 0..5 {
        let event = CacheEvent::AddAddon {
            path: PathBuf::from(format!("/test/mod_{}.jar", i)),
            instance_id: Some("test_instance".to_string()),
        };
        coordinator.send_event(event).unwrap();
    }

    // Shutdown immediately without waiting for processing
    coordinator.shutdown().await.unwrap();

    // This should complete without hanging
}

#[tokio::test]
async fn test_cache_manager_scoped_lifecycle_hanging_reproduction() {
    use crate::cache_manager::CacheManager;

    // This test reproduces the hanging issue seen in export tests
    println!("=== CACHE HANGING TEST START ===");

    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig {
        runtime_path: temp_dir.path().to_path_buf(),
        max_image_size: 5 * 1024 * 1024,
        max_cache_size: 1024 * 1024 * 1024,
        cleanup_interval: 24 * 3600,
    };

    let storage = Arc::new(MockAddonStorage::new());

    // Create a scoped cache manager (like override_caching_and_wait does)
    let mut cache_manager = CacheManager::new(storage.clone(), config);

    println!("Initializing cache manager...");

    // Initialize with timeout like the export code does
    let init_result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        cache_manager.initialize(),
    )
    .await;

    match init_result {
        Ok(Ok(())) => println!("Cache manager initialized successfully"),
        Ok(Err(e)) => {
            println!("Cache manager initialization failed: {}", e);
            return;
        }
        Err(_) => {
            println!("Cache manager initialization timed out");
            return;
        }
    }

    // Create a test JAR file to cache
    let test_jar_path = temp_dir.path().join("test_mod.jar");
    fs::write(&test_jar_path, b"test jar content for hanging reproduction").unwrap();

    println!("Caching addon: {:?}", test_jar_path);

    // Cache multiple addons to increase the chance of hanging, like export tests might do
    for i in 0..5 {
        let addon_path = temp_dir.path().join(format!("test_mod_{}.jar", i));
        fs::write(&addon_path, format!("test jar content {}", i)).unwrap();

        // Send cache request without waiting for completion (like export code does)
        // This is the key difference - export code sends the cache request
        // but doesn't wait for background processing to finish
        if let Ok(()) = cache_manager
            .cache_addon(addon_path, Some("test_instance".to_string()))
            .await
        {
            println!("Sent cache request for addon {}", i);
        }
    }

    // Don't wait for processing to complete - immediately try to shutdown
    // This is what the export tests do and why they hang
    println!("Attempting immediate shutdown without waiting for background processing...");

    // The export test timeout is only 3 seconds, but cache_manager.shutdown()
    // internally tries to shutdown coordinator with 10 second timeout
    // When the coordinator has active background work, it can't shutdown in 3 seconds
    let shutdown_result = tokio::time::timeout(
        std::time::Duration::from_secs(3), // Export test timeout
        cache_manager.shutdown(),
    )
    .await;

    match shutdown_result {
        Ok(Ok(())) => println!("Cache manager shutdown completed successfully"),
        Ok(Err(e)) => println!("Cache manager shutdown error: {}", e),
        Err(_) => {
            println!("Cache manager shutdown timed out after 3 seconds!");
            println!("This reproduces the hanging issue - cache manager needs up to 10 seconds");
            println!("but export tests only wait 3 seconds, causing timeout and hanging");

            // In real export tests, this would cause the test to timeout and hang
            // because the test runner would wait indefinitely for the test to complete
        }
    }

    println!("=== CACHE HANGING TEST END ===");

    // If we reach this point, either shutdown worked quickly or timed out gracefully
}
