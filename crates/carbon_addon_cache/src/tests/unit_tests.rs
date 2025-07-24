use crate::events::*;
use crate::hard_links::HardLinkManager;
use crate::notifier::ProgressNotifier;
use crate::storage::*;
use crate::tests::mock_storage::MockAddonStorage;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio_test;

#[tokio::test]
async fn test_basic_metadata_storage() {
    let storage = MockAddonStorage::new();

    let metadata = LocalMetadata {
        addon_id: "test_addon".to_string(),
        name: "Test Mod".to_string(),
        version: "1.0.0".to_string(),
        authors: vec!["TestAuthor".to_string()],
        description: Some("A test mod".to_string()),
        dependencies: vec![],
        checksums: Checksums {
            blake3: "abc123".to_string(),
            sha256: "def456".to_string(),
            md5: "ghi789".to_string(),
            murmur2: 12345,
        },
        mod_format: ModFormat::Fabric,
        minecraft_versions: vec!["1.20.1".to_string()],
        mod_loaders: vec!["fabric".to_string()],
        instance_id: Some("test_instance".to_string()),
    };

    // Store metadata
    storage
        .store_metadata("test_addon", &metadata)
        .await
        .unwrap();

    // Retrieve metadata
    let retrieved = storage.get_metadata("test_addon").await.unwrap();
    assert!(retrieved.is_some());

    let retrieved_metadata = retrieved.unwrap();
    assert_eq!(retrieved_metadata.name, "Test Mod");
    assert_eq!(retrieved_metadata.version, "1.0.0");
    assert_eq!(retrieved_metadata.authors, vec!["TestAuthor"]);
}

#[tokio::test]
async fn test_checksum_operations() {
    let storage = MockAddonStorage::new();

    let checksums = Checksums {
        blake3: "blake3_hash".to_string(),
        sha256: "sha256_hash".to_string(),
        md5: "md5_hash".to_string(),
        murmur2: 98765,
    };

    // Store checksums
    storage
        .store_checksums("test_addon", &checksums)
        .await
        .unwrap();

    // Retrieve checksums
    let retrieved = storage.get_checksums("test_addon").await.unwrap();
    assert!(retrieved.is_some());

    let retrieved_checksums = retrieved.unwrap();
    assert_eq!(retrieved_checksums.blake3, "blake3_hash");
    assert_eq!(retrieved_checksums.murmur2, 98765);

    // Test find by checksum
    let found = storage
        .find_by_checksum(ChecksumType::Blake3, "blake3_hash")
        .await
        .unwrap();
    assert_eq!(found, Some("test_addon".to_string()));

    let not_found = storage
        .find_by_checksum(ChecksumType::Blake3, "nonexistent")
        .await
        .unwrap();
    assert_eq!(not_found, None);
}

#[tokio::test]
async fn test_image_storage() {
    let storage = MockAddonStorage::new();

    let image_data = vec![0x89, 0x50, 0x4E, 0x47]; // PNG header bytes

    // Store image
    storage
        .store_image("test_addon", ImageType::Icon, &image_data)
        .await
        .unwrap();

    // Retrieve image
    let retrieved = storage
        .get_image("test_addon", ImageType::Icon)
        .await
        .unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap(), image_data);

    // List images
    let images = storage.list_images("test_addon").await.unwrap();
    assert!(images.contains(&ImageType::Icon));

    // Test mock helper
    assert!(storage.has_image("test_addon", ImageType::Icon));
    assert!(!storage.has_image("test_addon", ImageType::Gallery));
}

#[tokio::test]
async fn test_platform_data_storage() {
    let storage = MockAddonStorage::new();

    let platform_data = ModplatformData {
        platform: Platform::CurseForge,
        project_id: "123456".to_string(),
        file_id: "789012".to_string(),
        download_url: Some("https://example.com/mod.jar".to_string()),
        project_name: "Test Mod".to_string(),
        project_description: Some("A test mod from CurseForge".to_string()),
        categories: vec!["Technology".to_string()],
        license: Some("MIT".to_string()),
        website_url: Some("https://example.com".to_string()),
        source_url: Some("https://github.com/test/mod".to_string()),
        issues_url: Some("https://github.com/test/mod/issues".to_string()),
    };

    // Store platform data
    storage
        .store_platform_data("test_addon", &platform_data)
        .await
        .unwrap();

    // Retrieve platform data
    let retrieved = storage.get_platform_data("test_addon").await.unwrap();
    assert!(retrieved.is_some());

    let retrieved_data = retrieved.unwrap();
    assert_eq!(retrieved_data.platform, Platform::CurseForge);
    assert_eq!(retrieved_data.project_name, "Test Mod");
    assert_eq!(retrieved_data.categories, vec!["Technology"]);
}

#[tokio::test]
async fn test_version_storage() {
    let storage = MockAddonStorage::new();

    let versions = vec![
        Version {
            version_number: "2.0.0".to_string(),
            version_type: VersionType::Release,
            minecraft_versions: vec!["1.20.1".to_string()],
            mod_loaders: vec!["fabric".to_string()],
            release_date: "2024-01-01".to_string(),
            download_url: "https://example.com/v2.jar".to_string(),
            changelog: Some("Major update".to_string()),
        },
        Version {
            version_number: "1.5.0".to_string(),
            version_type: VersionType::Beta,
            minecraft_versions: vec!["1.20.0".to_string()],
            mod_loaders: vec!["fabric".to_string()],
            release_date: "2023-12-01".to_string(),
            download_url: "https://example.com/v1.5.jar".to_string(),
            changelog: Some("Beta release".to_string()),
        },
    ];

    // Store versions
    storage
        .store_versions("test_addon", &versions)
        .await
        .unwrap();

    // Retrieve versions
    let retrieved = storage.get_versions("test_addon").await.unwrap();
    assert_eq!(retrieved.len(), 2);
    assert_eq!(retrieved[0].version_number, "2.0.0");
    assert_eq!(retrieved[1].version_number, "1.5.0");

    // Get latest version
    let latest = storage.get_latest_version("test_addon").await.unwrap();
    assert!(latest.is_some());
    assert_eq!(latest.unwrap().version_number, "2.0.0");
}

#[tokio::test]
async fn test_instance_linking() {
    let storage = MockAddonStorage::new();

    let file_path = PathBuf::from("/instances/test/mods/test_mod.jar");

    // Link addon to instance
    storage
        .link_addon_to_instance("test_addon", "test_instance", &file_path)
        .await
        .unwrap();

    // Get instance addons
    let addons = storage.get_instance_addons("test_instance").await.unwrap();
    assert!(addons.contains(&"test_addon".to_string()));

    // Get addon instances
    let instances = storage.get_addon_instances("test_addon").await.unwrap();
    assert!(instances.contains(&"test_instance".to_string()));

    // Unlink addon from instance
    storage
        .unlink_addon_from_instance("test_addon", "test_instance")
        .await
        .unwrap();

    let addons_after = storage.get_instance_addons("test_instance").await.unwrap();
    assert!(!addons_after.contains(&"test_addon".to_string()));
}

#[tokio::test]
async fn test_hard_link_manager_creation() {
    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig {
        runtime_path: temp_dir.path().to_path_buf(),
        max_image_size: 1024 * 1024,
        max_cache_size: 100 * 1024 * 1024,
        cleanup_interval: 3600,
    };

    let mut manager = HardLinkManager::new(config);

    // Test start and shutdown
    manager.start().await.unwrap();

    // Verify addon directory was created
    let addon_dir = temp_dir.path().join("addons");
    assert!(addon_dir.exists());

    manager.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_progress_notifier() {
    let mut notifier = ProgressNotifier::new();
    notifier.start().await;

    // Subscribe to progress updates
    let _receiver = notifier.subscribe("test_instance".to_string());

    // Send progress update
    let update = crate::notifier::ProgressUpdate {
        instance_id: "test_instance".to_string(),
        stage: CacheStage::FileCache,
        current: 5,
        total: 10,
        completed: false,
        error: None,
    };

    notifier.update_progress(update);

    // Wait briefly for processing
    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

    // Check if we received progress
    let progress = notifier.get_instance_progress("test_instance");
    assert!(progress.is_some());

    // Add timeout to prevent hanging
    tokio::time::timeout(tokio::time::Duration::from_secs(2), notifier.shutdown())
        .await
        .expect("Shutdown should complete within 2 seconds");
}

#[tokio::test]
async fn test_priority_ordering() {
    // Test that priority enum ordering works correctly
    assert!(Priority::Critical > Priority::High);
    assert!(Priority::High > Priority::Normal);
    assert!(Priority::Normal > Priority::Low);

    let mut priorities = vec![
        Priority::Low,
        Priority::Critical,
        Priority::Normal,
        Priority::High,
    ];
    priorities.sort();

    assert_eq!(
        priorities,
        vec![
            Priority::Low,
            Priority::Normal,
            Priority::High,
            Priority::Critical
        ]
    );
}

#[tokio::test]
async fn test_event_serialization() {
    // Test that events can be serialized and deserialized
    let event = CacheEvent::AddAddon {
        path: PathBuf::from("/test/path"),
        instance_id: Some("test_instance".to_string()),
    };

    let serialized = serde_json::to_string(&event).unwrap();
    let deserialized: CacheEvent = serde_json::from_str(&serialized).unwrap();

    match deserialized {
        CacheEvent::AddAddon { path, instance_id } => {
            assert_eq!(path, PathBuf::from("/test/path"));
            assert_eq!(instance_id, Some("test_instance".to_string()));
        }
        _ => panic!("Event deserialization failed"),
    }
}

#[tokio::test]
async fn test_storage_config_clone() {
    let config = StorageConfig {
        runtime_path: PathBuf::from("/test"),
        max_image_size: 1024,
        max_cache_size: 2048,
        cleanup_interval: 3600,
    };

    let cloned = config.clone();
    assert_eq!(config.runtime_path, cloned.runtime_path);
    assert_eq!(config.max_image_size, cloned.max_image_size);
}

#[tokio::test]
async fn test_mock_storage_helpers() {
    let storage = MockAddonStorage::new();

    // Test initial state
    assert_eq!(storage.get_stored_metadata_count(), 0);
    assert_eq!(storage.get_stored_checksums_count(), 0);

    // Add some data
    let metadata = LocalMetadata {
        addon_id: "test".to_string(),
        name: "Test".to_string(),
        version: "1.0".to_string(),
        authors: vec![],
        description: None,
        dependencies: vec![],
        checksums: Checksums {
            blake3: "test".to_string(),
            sha256: "test".to_string(),
            md5: "test".to_string(),
            murmur2: 0,
        },
        mod_format: ModFormat::Unknown,
        minecraft_versions: vec![],
        mod_loaders: vec![],
        instance_id: Some("test_instance".to_string()),
    };

    storage.store_metadata("test", &metadata).await.unwrap();
    storage
        .store_checksums("test", &metadata.checksums)
        .await
        .unwrap();

    assert_eq!(storage.get_stored_metadata_count(), 1);
    assert_eq!(storage.get_stored_checksums_count(), 1);

    // Test clear
    storage.clear();
    assert_eq!(storage.get_stored_metadata_count(), 0);
    assert_eq!(storage.get_stored_checksums_count(), 0);
}
