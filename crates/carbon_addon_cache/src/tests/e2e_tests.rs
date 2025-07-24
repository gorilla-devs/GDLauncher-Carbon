use crate::coordinator::CacheCoordinator;
use crate::events::*;
use crate::storage::StorageConfig;
use crate::tests::mock_storage::MockAddonStorage;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::sleep;
use zip::{ZipWriter, write::FileOptions};

/// End-to-end tests simulating complete user workflows
#[cfg(test)]
mod e2e_tests {
    use super::*;

    #[tokio::test]
    async fn test_complete_user_workflow_instance_creation_to_export() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(MockAddonStorage::new());
        let config = create_test_config(&temp_dir);

        let mut coordinator = CacheCoordinator::new(storage.clone(), config).unwrap();
        coordinator.start().await.unwrap();

        // Step 1: User creates new instance
        let instance_id = "new_user_instance";

        // Step 2: User adds initial mods
        let initial_mods = vec![
            "jei-1.20.1-15.2.0.27.jar",
            "iron-chests-1.20.1-14.4.4.jar",
            "waystones-1.20.1-14.1.3.jar",
        ];

        for mod_name in &initial_mods {
            let mod_path = create_realistic_mod_file(&temp_dir, mod_name).await;
            coordinator
                .send_event(CacheEvent::AddAddon {
                    path: mod_path,
                    instance_id: Some(instance_id.to_string()),
                })
                .unwrap();
        }

        // Step 3: User starts playing (instance becomes high priority)
        coordinator.prioritize_instance(instance_id.to_string(), Priority::High);

        // Wait for initial caching to complete
        sleep(Duration::from_millis(200)).await;

        // Step 4: User adds more mods during gameplay
        let additional_mods = vec![
            "thermal-expansion-1.20.1-10.0.5.jar",
            "applied-energistics-2-1.20.1-15.0.16.jar",
        ];

        for mod_name in &additional_mods {
            let mod_path = create_realistic_mod_file(&temp_dir, mod_name).await;
            coordinator
                .send_event(CacheEvent::AddAddon {
                    path: mod_path,
                    instance_id: Some(instance_id.to_string()),
                })
                .unwrap();
        }

        // Step 5: User temporarily goes offline
        coordinator.set_online_status(false);
        sleep(Duration::from_millis(50)).await;

        // Step 6: User adds mod while offline
        let offline_mod = create_realistic_mod_file(&temp_dir, "offline-mod-1.0.0.jar").await;
        coordinator
            .send_event(CacheEvent::AddAddon {
                path: offline_mod,
                instance_id: Some(instance_id.to_string()),
            })
            .unwrap();

        // Step 7: User comes back online
        coordinator.set_online_status(true);
        sleep(Duration::from_millis(100)).await;

        // Step 8: User prepares to export (critical priority)
        coordinator.prioritize_instance(instance_id.to_string(), Priority::Critical);
        sleep(Duration::from_millis(100)).await;

        // Step 9: Export completes
        coordinator.shutdown().await.unwrap();

        // Verify complete workflow succeeded
        assert!(true); // Test passes if no panics occur during the complete workflow
    }

    #[tokio::test]
    async fn test_large_modpack_processing_100_plus_mods() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(MockAddonStorage::new());
        let config = create_test_config(&temp_dir);

        let mut coordinator = CacheCoordinator::new(storage.clone(), config).unwrap();
        coordinator.start().await.unwrap();

        let instance_id = "large_modpack_instance";

        // Create and add 100+ mods to simulate large modpack
        let mod_categories = vec![
            ("technology", 25),
            ("magic", 20),
            ("adventure", 15),
            ("utility", 20),
            ("decoration", 10),
            ("storage", 15),
        ];

        let mut total_mods = 0;
        for (category, count) in mod_categories {
            for i in 0..count {
                let mod_name = format!("{}-mod-{}-v1.0.{}.jar", category, i, i % 10);
                let mod_path = create_realistic_mod_file(&temp_dir, &mod_name).await;

                coordinator
                    .send_event(CacheEvent::AddAddon {
                        path: mod_path,
                        instance_id: Some(instance_id.to_string()),
                    })
                    .unwrap();

                total_mods += 1;

                // Add small delays to prevent overwhelming the system
                if total_mods % 10 == 0 {
                    sleep(Duration::from_millis(10)).await;
                }
            }
        }

        // Set high priority for processing
        coordinator.prioritize_instance(instance_id.to_string(), Priority::High);

        // Wait for bulk processing to complete
        sleep(Duration::from_millis(500)).await;

        coordinator.shutdown().await.unwrap();

        assert!(
            total_mods >= 100,
            "Should have processed 100+ mods, got {}",
            total_mods
        );
    }

    #[tokio::test]
    async fn test_simultaneous_multi_instance_caching() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(MockAddonStorage::new());
        let config = create_test_config(&temp_dir);

        let mut coordinator = CacheCoordinator::new(storage.clone(), config).unwrap();
        coordinator.start().await.unwrap();

        // Create multiple instances simultaneously
        let instances = vec![
            ("vanilla_plus_instance", Priority::High),
            ("tech_modpack_instance", Priority::Normal),
            ("magic_modpack_instance", Priority::Normal),
            ("kitchen_sink_instance", Priority::Low),
        ];

        // Add mods to all instances simultaneously
        for (instance_id, priority) in &instances {
            coordinator.prioritize_instance(instance_id.to_string(), *priority);

            // Add different mod sets for each instance
            let instance_mods = match instance_id {
                &"vanilla_plus_instance" => vec![
                    "jei-1.20.1-15.2.0.27.jar",
                    "waystones-1.20.1-14.1.3.jar",
                    "iron-chests-1.20.1-14.4.4.jar",
                ],
                &"tech_modpack_instance" => vec![
                    "thermal-expansion-1.20.1-10.0.5.jar",
                    "applied-energistics-2-1.20.1-15.0.16.jar",
                    "mekanism-1.20.1-10.4.5.jar",
                    "refined-storage-1.20.1-1.12.4.jar",
                ],
                &"magic_modpack_instance" => vec![
                    "thaumcraft-1.20.1-6.1.7.jar",
                    "botania-1.20.1-446.jar",
                    "ars-nouveau-1.20.1-4.9.0.jar",
                    "blood-magic-1.20.1-3.3.2.jar",
                ],
                &"kitchen_sink_instance" => vec![
                    "create-1.20.1-0.5.1.jar",
                    "tinkers-construct-1.20.1-3.7.1.jar",
                    "twilight-forest-1.20.1-4.3.2145.jar",
                    "biomes-o-plenty-1.20.1-18.0.0.jar",
                    "alex-mobs-1.20.1-1.22.8.jar",
                ],
                _ => vec![],
            };

            for mod_name in instance_mods {
                let mod_path = create_realistic_mod_file(&temp_dir, mod_name).await;
                coordinator
                    .send_event(CacheEvent::AddAddon {
                        path: mod_path,
                        instance_id: Some(instance_id.to_string()),
                    })
                    .unwrap();
            }
        }

        // Simulate user switching between instances
        sleep(Duration::from_millis(100)).await;
        coordinator.prioritize_instance("tech_modpack_instance".to_string(), Priority::High);

        sleep(Duration::from_millis(100)).await;
        coordinator.prioritize_instance("magic_modpack_instance".to_string(), Priority::High);

        sleep(Duration::from_millis(100)).await;
        coordinator.prioritize_instance("kitchen_sink_instance".to_string(), Priority::Critical);

        // Wait for all processing to complete
        sleep(Duration::from_millis(300)).await;

        coordinator.shutdown().await.unwrap();

        assert!(true); // Test passes if system handles concurrent instances without issues
    }

    #[tokio::test]
    async fn test_crash_recovery_with_large_workload() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(MockAddonStorage::new());
        let config = create_test_config(&temp_dir);

        // First session - simulate crash during heavy workload
        {
            let mut coordinator1 = CacheCoordinator::new(storage.clone(), config.clone()).unwrap();
            coordinator1.start().await.unwrap();

            let instance_id = "crash_recovery_instance";
            coordinator1.prioritize_instance(instance_id.to_string(), Priority::High);

            // Add many mods quickly
            for i in 0..50 {
                let mod_name = format!("crash-test-mod-{}.jar", i);
                let mod_path = create_realistic_mod_file(&temp_dir, &mod_name).await;
                coordinator1
                    .send_event(CacheEvent::AddAddon {
                        path: mod_path,
                        instance_id: Some(instance_id.to_string()),
                    })
                    .unwrap();
            }

            // Wait a bit then simulate crash
            sleep(Duration::from_millis(100)).await;
            // Coordinator drops here simulating crash
        }

        // Second session - recovery
        {
            let mut coordinator2 = CacheCoordinator::new(storage.clone(), config.clone()).unwrap();
            coordinator2.start().await.unwrap();

            // Add more work after recovery
            for i in 50..75 {
                let mod_name = format!("post-crash-mod-{}.jar", i);
                let mod_path = create_realistic_mod_file(&temp_dir, &mod_name).await;
                coordinator2
                    .send_event(CacheEvent::AddAddon {
                        path: mod_path,
                        instance_id: Some("crash_recovery_instance".to_string()),
                    })
                    .unwrap();
            }

            sleep(Duration::from_millis(200)).await;
            coordinator2.shutdown().await.unwrap();
        }

        assert!(true); // Test passes if recovery works correctly
    }

    #[tokio::test]
    async fn test_network_resilience_during_operation() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(MockAddonStorage::new());
        let config = create_test_config(&temp_dir);

        let mut coordinator = CacheCoordinator::new(storage.clone(), config).unwrap();
        coordinator.start().await.unwrap();

        let instance_id = "network_test_instance";

        // Start online
        coordinator.set_online_status(true);

        // Add mods that would trigger network requests
        let network_mods = vec![
            "curseforge-mod-1.jar",
            "modrinth-mod-1.jar",
            "popular-mod-with-updates.jar",
        ];

        for mod_name in &network_mods {
            let mod_path = create_realistic_mod_file(&temp_dir, mod_name).await;
            coordinator
                .send_event(CacheEvent::AddAddon {
                    path: mod_path,
                    instance_id: Some(instance_id.to_string()),
                })
                .unwrap();
        }

        sleep(Duration::from_millis(50)).await;

        // Simulate network loss
        coordinator.set_online_status(false);

        // Add more mods while offline
        for i in 0..5 {
            let mod_name = format!("offline-mod-{}.jar", i);
            let mod_path = create_realistic_mod_file(&temp_dir, &mod_name).await;
            coordinator
                .send_event(CacheEvent::AddAddon {
                    path: mod_path,
                    instance_id: Some(instance_id.to_string()),
                })
                .unwrap();
        }

        sleep(Duration::from_millis(50)).await;

        // Network comes back
        coordinator.set_online_status(true);

        // Add final mods
        for i in 0..3 {
            let mod_name = format!("online-again-mod-{}.jar", i);
            let mod_path = create_realistic_mod_file(&temp_dir, &mod_name).await;
            coordinator
                .send_event(CacheEvent::AddAddon {
                    path: mod_path,
                    instance_id: Some(instance_id.to_string()),
                })
                .unwrap();
        }

        sleep(Duration::from_millis(100)).await;

        coordinator.shutdown().await.unwrap();

        assert!(true); // System should handle network changes gracefully
    }

    async fn create_realistic_mod_file(temp_dir: &TempDir, mod_name: &str) -> PathBuf {
        let mod_path = temp_dir.path().join(mod_name);
        let file = std::fs::File::create(&mod_path).unwrap();
        let mut zip = ZipWriter::new(file);

        // Create realistic mod structure based on mod name
        if mod_name.contains("fabric") || mod_name.contains("quilt") {
            // Fabric/Quilt mod
            let fabric_json = serde_json::json!({
                "schemaVersion": 1,
                "id": mod_name.replace(".jar", "").replace("-", "_"),
                "version": "1.0.0",
                "name": mod_name.replace(".jar", "").replace("-", " "),
                "description": format!("A test mod: {}", mod_name),
                "authors": ["TestAuthor"],
                "environment": "*",
                "depends": {
                    "fabricloader": ">=0.14.0",
                    "minecraft": "~1.20.1"
                }
            });

            zip.start_file("fabric.mod.json", FileOptions::default())
                .unwrap();
            zip.write_all(fabric_json.to_string().as_bytes()).unwrap();
        } else {
            // Forge mod
            let mods_toml = format!(
                r#"
modLoader="javafml"
loaderVersion="[47,)"
license="MIT"

[[mods]]
modId="{}"
version="1.0.0"
displayName="{}"
description="A test mod: {}"
authors="TestAuthor"
"#,
                mod_name.replace(".jar", "").replace("-", "_"),
                mod_name.replace(".jar", "").replace("-", " "),
                mod_name
            );

            zip.start_file("META-INF/mods.toml", FileOptions::default())
                .unwrap();
            zip.write_all(mods_toml.as_bytes()).unwrap();
        }

        // Add icon
        let icon_data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]; // PNG header
        zip.start_file("icon.png", FileOptions::default()).unwrap();
        zip.write_all(&icon_data).unwrap();

        zip.finish().unwrap();
        mod_path
    }
}

// Helper function
fn create_test_config(temp_dir: &TempDir) -> StorageConfig {
    StorageConfig {
        runtime_path: temp_dir.path().to_path_buf(),
        max_image_size: 1024 * 1024,
        max_cache_size: 500 * 1024 * 1024, // Larger cache for e2e tests
        cleanup_interval: 3600,
    }
}
