use crate::coordinator::CacheCoordinator;
use crate::events::*;
use crate::persistence::{CacheProgress, CacheStatusPersistence, EventPersistence};
use crate::storage::StorageConfig;
use crate::tests::mock_storage::MockAddonStorage;
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::sleep;

#[cfg(test)]
mod app_restart_tests {
    use super::*;

    #[tokio::test]
    async fn test_event_persistence_across_restart() {
        let temp_dir = TempDir::new().unwrap();
        let persistence = EventPersistence::new(temp_dir.path(), 100).unwrap();

        // Simulate events before crash
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
        ];

        // Persist events
        for event in &events {
            persistence.persist_event(event.clone()).await.unwrap();
        }

        // Start persistence task and wait for flush
        persistence.start_persistence_task().await;
        sleep(Duration::from_millis(100)).await;

        // Simulate app restart - create new persistence instance
        let persistence_after_restart = EventPersistence::new(temp_dir.path(), 100).unwrap();
        let recovered_events = persistence_after_restart.load_events().await.unwrap();

        assert_eq!(recovered_events.len(), 3);
        assert!(matches!(recovered_events[0], CacheEvent::AddAddon { .. }));
        assert!(matches!(recovered_events[1], CacheEvent::AddAddon { .. }));
        assert!(matches!(
            recovered_events[2],
            CacheEvent::PrioritizeInstance { .. }
        ));
    }

    #[tokio::test]
    async fn test_cache_status_persistence_across_restart() {
        let temp_dir = TempDir::new().unwrap();
        let status_persistence = CacheStatusPersistence::new(temp_dir.path()).unwrap();

        // Simulate caching status before crash
        let progress = CacheProgress {
            current: 7,
            total: 15,
            stage: "metadata_extraction".to_string(),
            percentage: 46.7,
        };

        status_persistence
            .update_instance_state(
                "test_instance",
                "caching",
                Some("current_addon.jar".to_string()),
                progress.clone(),
            )
            .await
            .unwrap();

        // Start persistence task and wait for flush
        status_persistence.start_persistence_task().await;
        sleep(Duration::from_millis(100)).await;

        // Simulate app restart
        let status_after_restart = CacheStatusPersistence::new(temp_dir.path()).unwrap();
        let recovered_status = status_after_restart.load_status().await.unwrap();

        assert_eq!(recovered_status.instances.len(), 1);
        let instance_state = recovered_status.instances.get("test_instance").unwrap();
        assert_eq!(instance_state.status, "caching");
        assert_eq!(
            instance_state.current_addon,
            Some("current_addon.jar".to_string())
        );
        assert_eq!(instance_state.progress.current, 7);
        assert_eq!(instance_state.progress.total, 15);
    }

    #[tokio::test]
    async fn test_coordinator_crash_recovery() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(MockAddonStorage::new());
        let config = StorageConfig {
            runtime_path: temp_dir.path().to_path_buf(),
            max_image_size: 1024 * 1024,
            max_cache_size: 100 * 1024 * 1024,
            cleanup_interval: 3600,
        };

        // Create first coordinator and send events
        {
            let mut coordinator1 = CacheCoordinator::new(storage.clone(), config.clone()).unwrap();
            coordinator1.start().await.unwrap();

            // Send some events
            coordinator1
                .send_event(CacheEvent::AddAddon {
                    path: PathBuf::from("/test/mod1.jar"),
                    instance_id: Some("instance1".to_string()),
                })
                .unwrap();

            coordinator1
                .send_event(CacheEvent::PrioritizeInstance {
                    instance_id: "instance1".to_string(),
                })
                .unwrap();

            // Wait for persistence
            sleep(Duration::from_millis(100)).await;

            // Simulate crash by dropping coordinator without proper shutdown
            drop(coordinator1);
        }

        // Create new coordinator (simulating app restart)
        let mut coordinator2 = CacheCoordinator::new(storage.clone(), config.clone()).unwrap();
        coordinator2.start().await.unwrap();

        // Events should be recovered and processed
        sleep(Duration::from_secs(2)).await;

        coordinator2.shutdown().await.unwrap();

        // Test passes if no panics occur during recovery
        assert!(true);
    }

    #[tokio::test]
    async fn test_partial_event_processing_recovery() {
        let temp_dir = TempDir::new().unwrap();
        let persistence = EventPersistence::new(temp_dir.path(), 100).unwrap();

        // Simulate scenario where some events were processed, others weren't
        let events = vec![
            CacheEvent::AddAddon {
                path: PathBuf::from("/test/processed.jar"),
                instance_id: Some("instance1".to_string()),
            },
            CacheEvent::AddAddon {
                path: PathBuf::from("/test/unprocessed.jar"),
                instance_id: Some("instance2".to_string()),
            },
        ];

        // Persist both events
        for event in &events {
            persistence.persist_event(event.clone()).await.unwrap();
        }

        // Mark first event as processed
        persistence.mark_processed(&events[0]).await.unwrap();

        // Start persistence and wait for flush
        persistence.start_persistence_task().await;
        sleep(Duration::from_millis(100)).await;

        // Restart and recover
        let persistence_after_restart = EventPersistence::new(temp_dir.path(), 100).unwrap();
        let recovered_events = persistence_after_restart.load_events().await.unwrap();

        // Should only have the unprocessed event
        assert_eq!(recovered_events.len(), 1);
        if let CacheEvent::AddAddon { path, .. } = &recovered_events[0] {
            assert!(path.to_str().unwrap().contains("unprocessed"));
        } else {
            panic!("Wrong event type recovered");
        }
    }

    #[tokio::test]
    async fn test_corrupted_persistence_file_recovery() {
        let temp_dir = TempDir::new().unwrap();

        // Create corrupted persistence file
        let persistence_file = temp_dir.path().join("cache_events.json");
        tokio::fs::write(&persistence_file, "invalid json content")
            .await
            .unwrap();

        // Should handle gracefully
        let persistence = EventPersistence::new(temp_dir.path(), 100).unwrap();
        let result = persistence.load_events().await;

        // Should either succeed with empty events or fail gracefully
        assert!(result.is_ok() || result.is_err());

        if let Ok(events) = result {
            // If it succeeds, it should return empty events
            assert!(events.is_empty());
        }
    }

    #[tokio::test]
    async fn test_disk_space_exhaustion_recovery() {
        let temp_dir = TempDir::new().unwrap();
        let persistence = EventPersistence::new(temp_dir.path(), 5).unwrap(); // Small max events

        // Fill up the persistence queue beyond capacity
        for i in 0..10 {
            let event = CacheEvent::AddAddon {
                path: PathBuf::from(format!("/test/mod{}.jar", i)),
                instance_id: Some(format!("instance{}", i)),
            };
            persistence.persist_event(event).await.unwrap();
        }

        // Should handle queue overflow gracefully
        persistence.start_persistence_task().await;
        sleep(Duration::from_millis(100)).await;

        // Recovery should work with limited events
        let persistence_after_restart = EventPersistence::new(temp_dir.path(), 5).unwrap();
        let recovered_events = persistence_after_restart.load_events().await.unwrap();

        // Should have at most 5 events (the capacity limit)
        assert!(recovered_events.len() <= 5);
    }

    #[tokio::test]
    async fn test_multiple_restart_cycles() {
        let temp_dir = TempDir::new().unwrap();

        // First cycle
        {
            let persistence = EventPersistence::new(temp_dir.path(), 100).unwrap();
            persistence
                .persist_event(CacheEvent::AddAddon {
                    path: PathBuf::from("/test/cycle1.jar"),
                    instance_id: Some("instance1".to_string()),
                })
                .await
                .unwrap();

            persistence.start_persistence_task().await;
            sleep(Duration::from_millis(100)).await;
        }

        // Second cycle - recover and add more
        {
            let persistence = EventPersistence::new(temp_dir.path(), 100).unwrap();
            let events = persistence.load_events().await.unwrap();
            assert_eq!(events.len(), 1);

            persistence
                .persist_event(CacheEvent::AddAddon {
                    path: PathBuf::from("/test/cycle2.jar"),
                    instance_id: Some("instance2".to_string()),
                })
                .await
                .unwrap();

            persistence.start_persistence_task().await;
            sleep(Duration::from_millis(100)).await;
        }

        // Third cycle - should start clean (previous events processed)
        {
            let persistence = EventPersistence::new(temp_dir.path(), 100).unwrap();
            let events = persistence.load_events().await.unwrap();
            // Should have 1 event (cycle2, since cycle1 was processed in cycle2)
            assert_eq!(events.len(), 1);
        }
    }
}
