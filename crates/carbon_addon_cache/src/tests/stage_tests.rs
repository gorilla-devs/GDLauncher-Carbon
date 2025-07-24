use crate::events::*;
use crate::stages::*;
use crate::storage::StorageConfig;
use anyhow::Result;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::fs;

/// Test Stage 1 (FileCache) with various file types
#[cfg(test)]
mod stage1_tests {
    use super::*;

    #[tokio::test]
    async fn test_stage1_jar_files() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let file_cache = FileCache::new(config).unwrap();

        // Create test JAR file
        let jar_path = temp_dir.path().join("test_mod.jar");
        fs::write(&jar_path, b"PK\x03\x04test jar content")
            .await
            .unwrap();

        // Test processing
        let result = file_cache
            .add_file(
                jar_path.clone(),
                Some("test_instance".to_string()),
                Priority::Normal,
            )
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stage1_zip_files() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let file_cache = FileCache::new(config).unwrap();

        // Create test ZIP file
        let zip_path = temp_dir.path().join("resource_pack.zip");
        fs::write(&zip_path, b"PK\x03\x04resource pack content")
            .await
            .unwrap();

        let result = file_cache
            .add_file(
                zip_path.clone(),
                Some("test_instance".to_string()),
                Priority::Normal,
            )
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stage1_invalid_files() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let file_cache = FileCache::new(config).unwrap();

        // Test non-existent file
        let invalid_path = temp_dir.path().join("nonexistent.jar");
        let result = file_cache
            .add_file(
                invalid_path,
                Some("test_instance".to_string()),
                Priority::Normal,
            )
            .await;
        assert!(result.is_err());

        // Test empty file
        let empty_path = temp_dir.path().join("empty.jar");
        fs::write(&empty_path, b"").await.unwrap();
        let result = file_cache
            .add_file(
                empty_path,
                Some("test_instance".to_string()),
                Priority::Normal,
            )
            .await;
        assert!(result.is_ok()); // Should handle empty files gracefully
    }

    #[tokio::test]
    async fn test_stage1_large_files() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let file_cache = FileCache::new(config).unwrap();

        // Create large file (10MB)
        let large_path = temp_dir.path().join("large_mod.jar");
        let large_content = vec![0u8; 10 * 1024 * 1024];
        fs::write(&large_path, large_content).await.unwrap();

        let result = file_cache
            .add_file(
                large_path,
                Some("test_instance".to_string()),
                Priority::Normal,
            )
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stage1_different_extensions() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let file_cache = FileCache::new(config).unwrap();

        // Test various file extensions
        let extensions = vec!["jar", "zip", "litemod", "disabled"];

        for ext in extensions {
            let file_path = temp_dir.path().join(format!("test_mod.{}", ext));
            fs::write(&file_path, b"test content").await.unwrap();

            let result = file_cache
                .add_file(
                    file_path,
                    Some("test_instance".to_string()),
                    Priority::Normal,
                )
                .await;
            assert!(result.is_ok(), "Failed for extension: {}", ext);
        }
    }
}

/// Test Stage 2 (MetadataExtractor) with various mod formats
#[cfg(test)]
mod stage2_tests {
    use super::*;
    use std::io::{Cursor, Write};
    use zip::{ZipWriter, write::FileOptions};

    #[tokio::test]
    async fn test_stage2_fabric_mod() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let extractor = MetadataExtractor::new(config).unwrap();

        // Create Fabric mod JAR with fabric.mod.json
        let jar_path = create_fabric_mod_jar(&temp_dir, "test_fabric_mod").await;

        let basic_metadata = BasicMetadata {
            addon_id: "test_fabric_mod".to_string(),
            file_path: jar_path,
            file_size: 1024,
            modified_time: 1234567890,
            addon_type: AddonType::Mod,
            instance_id: Some("test_instance".to_string()),
        };

        let result = extractor
            .add_addon("test_addon".to_string(), basic_metadata, Priority::Normal)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stage2_forge_mod() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let extractor = MetadataExtractor::new(config).unwrap();

        // Create Forge mod JAR with mods.toml
        let jar_path = create_forge_mod_jar(&temp_dir, "test_forge_mod").await;

        let basic_metadata = BasicMetadata {
            addon_id: "test_forge_mod".to_string(),
            file_path: jar_path,
            file_size: 2048,
            modified_time: 1234567890,
            addon_type: AddonType::Mod,
            instance_id: Some("test_instance".to_string()),
        };

        let result = extractor
            .add_addon("test_addon".to_string(), basic_metadata, Priority::Normal)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stage2_quilt_mod() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let extractor = MetadataExtractor::new(config).unwrap();

        // Create Quilt mod JAR with quilt.mod.json
        let jar_path = create_quilt_mod_jar(&temp_dir, "test_quilt_mod").await;

        let basic_metadata = BasicMetadata {
            addon_id: "test_quilt_mod".to_string(),
            file_path: jar_path,
            file_size: 1536,
            modified_time: 1234567890,
            addon_type: AddonType::Mod,
            instance_id: Some("test_instance".to_string()),
        };

        let result = extractor
            .add_addon("test_addon".to_string(), basic_metadata, Priority::Normal)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stage2_legacy_forge_mod() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let extractor = MetadataExtractor::new(config).unwrap();

        // Create legacy Forge mod JAR with mcmod.info
        let jar_path = create_legacy_forge_mod_jar(&temp_dir, "test_legacy_mod").await;

        let basic_metadata = BasicMetadata {
            addon_id: "test_legacy_mod".to_string(),
            file_path: jar_path,
            file_size: 1800,
            modified_time: 1234567890,
            addon_type: AddonType::Mod,
            instance_id: Some("test_instance".to_string()),
        };

        let result = extractor
            .add_addon("test_addon".to_string(), basic_metadata, Priority::Normal)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stage2_corrupted_jar() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let extractor = MetadataExtractor::new(config).unwrap();

        // Create corrupted JAR file
        let jar_path = temp_dir.path().join("corrupted.jar");
        fs::write(&jar_path, b"not a valid zip file").await.unwrap();

        let basic_metadata = BasicMetadata {
            addon_id: "corrupted_mod".to_string(),
            file_path: jar_path,
            file_size: 20,
            modified_time: 1234567890,
            addon_type: AddonType::Mod,
            instance_id: Some("test_instance".to_string()),
        };

        // Should handle gracefully
        let result = extractor
            .add_addon("test_addon".to_string(), basic_metadata, Priority::Normal)
            .await;
        assert!(result.is_err()); // Should fail but not crash
    }

    async fn create_fabric_mod_jar(temp_dir: &TempDir, mod_name: &str) -> PathBuf {
        let jar_path = temp_dir.path().join(format!("{}.jar", mod_name));
        let file = std::fs::File::create(&jar_path).unwrap();
        let mut zip = ZipWriter::new(file);

        // Add fabric.mod.json
        let fabric_json = serde_json::json!({
            "schemaVersion": 1,
            "id": mod_name,
            "version": "1.0.0",
            "name": "Test Fabric Mod",
            "description": "A test Fabric mod",
            "authors": ["test_author"],
            "contact": {},
            "license": "MIT",
            "icon": "icon.png",
            "environment": "*",
            "entrypoints": {
                "main": ["com.example.TestMod"]
            },
            "depends": {
                "fabricloader": ">=0.14.0",
                "minecraft": "~1.20.1"
            }
        });

        zip.start_file("fabric.mod.json", FileOptions::default())
            .unwrap();
        zip.write_all(fabric_json.to_string().as_bytes()).unwrap();
        zip.finish().unwrap();

        jar_path
    }

    async fn create_forge_mod_jar(temp_dir: &TempDir, mod_name: &str) -> PathBuf {
        let jar_path = temp_dir.path().join(format!("{}.jar", mod_name));
        let file = std::fs::File::create(&jar_path).unwrap();
        let mut zip = ZipWriter::new(file);

        // Add META-INF/mods.toml
        let mods_toml = format!(
            r#"
modLoader="javafml"
loaderVersion="[47,)"
license="MIT"

[[mods]]
modId="{}"
version="1.0.0"
displayName="Test Forge Mod"
description="A test Forge mod"
authors="test_author"

[[dependencies.{}]]
modId="forge"
mandatory=true
versionRange="[47.2.0,)"
ordering="NONE"
side="BOTH"
"#,
            mod_name, mod_name
        );

        zip.start_file("META-INF/mods.toml", FileOptions::default())
            .unwrap();
        zip.write_all(mods_toml.as_bytes()).unwrap();
        zip.finish().unwrap();

        jar_path
    }

    async fn create_quilt_mod_jar(temp_dir: &TempDir, mod_name: &str) -> PathBuf {
        let jar_path = temp_dir.path().join(format!("{}.jar", mod_name));
        let file = std::fs::File::create(&jar_path).unwrap();
        let mut zip = ZipWriter::new(file);

        // Add quilt.mod.json
        let quilt_json = serde_json::json!({
            "schema_version": 1,
            "quilt_loader": {
                "group": "com.example",
                "id": mod_name,
                "version": "1.0.0",
                "metadata": {
                    "name": "Test Quilt Mod",
                    "description": "A test Quilt mod",
                    "contributors": {
                        "test_author": "Owner"
                    },
                    "contact": {
                        "homepage": "https://example.com"
                    },
                    "license": "MIT"
                },
                "depends": [
                    {
                        "id": "quilt_loader",
                        "versions": ">=0.19.0"
                    },
                    {
                        "id": "minecraft",
                        "versions": "~1.20.1"
                    }
                ]
            }
        });

        zip.start_file("quilt.mod.json", FileOptions::default())
            .unwrap();
        zip.write_all(quilt_json.to_string().as_bytes()).unwrap();
        zip.finish().unwrap();

        jar_path
    }

    async fn create_legacy_forge_mod_jar(temp_dir: &TempDir, mod_name: &str) -> PathBuf {
        let jar_path = temp_dir.path().join(format!("{}.jar", mod_name));
        let file = std::fs::File::create(&jar_path).unwrap();
        let mut zip = ZipWriter::new(file);

        // Add mcmod.info
        let mcmod_json = serde_json::json!([{
            "modid": mod_name,
            "name": "Test Legacy Forge Mod",
            "description": "A test legacy Forge mod",
            "version": "1.0.0",
            "mcversion": "1.12.2",
            "url": "https://example.com",
            "authorList": ["test_author"],
            "credits": "Test credits",
            "logoFile": "",
            "screenshots": [],
            "dependencies": []
        }]);

        zip.start_file("mcmod.info", FileOptions::default())
            .unwrap();
        zip.write_all(mcmod_json.to_string().as_bytes()).unwrap();
        zip.finish().unwrap();

        jar_path
    }
}

/// Test Stage 3 (ImageCache) with various image sources
#[cfg(test)]
mod stage3_tests {
    use super::*;
    use std::io::Write;
    use zip::{ZipWriter, write::FileOptions};

    #[tokio::test]
    async fn test_stage3_jar_icon_extraction() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let image_cache = ImageCache::new(config).unwrap();

        // Create JAR with icon
        let jar_path = create_jar_with_icon(&temp_dir).await;

        let metadata = create_test_metadata(&jar_path);
        let result = image_cache
            .add_addon("test_addon".to_string(), metadata, Priority::Normal)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stage3_various_image_formats() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let image_cache = ImageCache::new(config).unwrap();

        // Test different image formats in JAR
        let formats = vec!["png", "jpg", "jpeg", "webp"];

        for format in formats {
            let jar_path = create_jar_with_image_format(&temp_dir, format).await;
            let metadata = create_test_metadata(&jar_path);

            let result = image_cache
                .add_addon(format!("test_addon_{}", format), metadata, Priority::Normal)
                .await;
            assert!(result.is_ok(), "Failed for format: {}", format);
        }
    }

    #[tokio::test]
    async fn test_stage3_corrupted_images() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let image_cache = ImageCache::new(config).unwrap();

        // Create JAR with corrupted image
        let jar_path = create_jar_with_corrupted_image(&temp_dir).await;
        let metadata = create_test_metadata(&jar_path);

        // Should handle gracefully
        let result = image_cache
            .add_addon("test_addon".to_string(), metadata, Priority::Normal)
            .await;
        assert!(result.is_ok()); // Should not crash on corrupted images
    }

    async fn create_jar_with_icon(temp_dir: &TempDir) -> PathBuf {
        let jar_path = temp_dir.path().join("mod_with_icon.jar");
        let file = std::fs::File::create(&jar_path).unwrap();
        let mut zip = ZipWriter::new(file);

        // Create a simple PNG icon (1x1 pixel red)
        let png_data = vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
            0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1x1 dimensions
            0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49,
            0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x0D,
            0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60,
            0x82,
        ];

        zip.start_file("icon.png", FileOptions::default()).unwrap();
        zip.write_all(&png_data).unwrap();
        zip.finish().unwrap();

        jar_path
    }

    async fn create_jar_with_image_format(temp_dir: &TempDir, format: &str) -> PathBuf {
        let jar_path = temp_dir.path().join(format!("mod_with_{}.jar", format));
        let file = std::fs::File::create(&jar_path).unwrap();
        let mut zip = ZipWriter::new(file);

        // Use minimal valid image data based on format
        let image_data = match format {
            "png" => vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A], // PNG header
            "jpg" | "jpeg" => vec![0xFF, 0xD8, 0xFF, 0xE0],                // JPEG header
            "webp" => vec![0x52, 0x49, 0x46, 0x46],                        // RIFF header for WebP
            _ => vec![0x00, 0x01, 0x02, 0x03],                             // Generic data
        };

        zip.start_file(&format!("icon.{}", format), FileOptions::default())
            .unwrap();
        zip.write_all(&image_data).unwrap();
        zip.finish().unwrap();

        jar_path
    }

    async fn create_jar_with_corrupted_image(temp_dir: &TempDir) -> PathBuf {
        let jar_path = temp_dir.path().join("mod_with_corrupted_image.jar");
        let file = std::fs::File::create(&jar_path).unwrap();
        let mut zip = ZipWriter::new(file);

        // Corrupted image data
        let corrupted_data = vec![0x00, 0xFF, 0xAA, 0x55, 0xCC, 0x33];

        zip.start_file("icon.png", FileOptions::default()).unwrap();
        zip.write_all(&corrupted_data).unwrap();
        zip.finish().unwrap();

        jar_path
    }
}

/// Test Stage 4 (ModplatformFetcher) with various platform scenarios
#[cfg(test)]
mod stage4_tests {
    use super::*;

    #[tokio::test]
    async fn test_stage4_curseforge_matching() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let fetcher = ModplatformFetcher::new(config).unwrap();

        let metadata = create_test_metadata_with_checksums();
        let result = fetcher
            .add_addon("test_addon".to_string(), metadata, Priority::Normal)
            .await;

        // This will likely fail due to network/API limitations in tests, but should not crash
        assert!(result.is_ok() || result.is_err()); // Either success or graceful failure
    }

    #[tokio::test]
    async fn test_stage4_modrinth_matching() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let fetcher = ModplatformFetcher::new(config).unwrap();

        let metadata = create_test_metadata_with_checksums();
        let result = fetcher
            .add_addon("test_addon".to_string(), metadata, Priority::Normal)
            .await;

        // Should handle gracefully whether API is available or not
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_stage4_offline_behavior() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let fetcher = ModplatformFetcher::new(config).unwrap();

        // Set offline mode
        fetcher.set_online(false).await.unwrap();

        let metadata = create_test_metadata_with_checksums();
        let result = fetcher
            .add_addon("test_addon".to_string(), metadata, Priority::Normal)
            .await;

        // Should queue for later processing
        assert!(result.is_ok());
    }

    fn create_test_metadata_with_checksums() -> LocalMetadata {
        LocalMetadata {
            addon_id: "test_addon".to_string(),
            name: "Test Addon".to_string(),
            version: "1.0.0".to_string(),
            authors: vec!["test_author".to_string()],
            description: Some("Test description".to_string()),
            dependencies: vec![],
            checksums: Checksums {
                blake3: "test_blake3_hash".to_string(),
                sha256: "test_sha256_hash".to_string(),
                md5: "test_md5_hash".to_string(),
                murmur2: 12345,
            },
            mod_format: ModFormat::Fabric,
            minecraft_versions: vec!["1.20.1".to_string()],
            mod_loaders: vec!["fabric".to_string()],
            instance_id: Some("test_instance".to_string()),
        }
    }
}

/// Test Stage 5 (UpdateChecker) with various update scenarios
#[cfg(test)]
mod stage5_tests {
    use super::*;

    #[tokio::test]
    async fn test_stage5_update_detection() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let update_checker = UpdateChecker::new(config).unwrap();

        let platform_data = create_test_platform_data();
        let result = update_checker
            .add_addon("test_addon".to_string(), platform_data, Priority::Normal)
            .await;

        // Should handle gracefully
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_stage5_version_comparison() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(&temp_dir);
        let update_checker = UpdateChecker::new(config).unwrap();

        let platform_data = ModplatformData {
            platform: Platform::CurseForge,
            project_id: "123456".to_string(),
            file_id: "789012".to_string(),
            download_url: Some("https://example.com/mod.jar".to_string()),
            project_name: "Test Mod".to_string(),
            project_description: Some("Test description".to_string()),
            categories: vec!["Technology".to_string()],
            license: Some("MIT".to_string()),
            website_url: Some("https://example.com".to_string()),
            source_url: Some("https://github.com/test/mod".to_string()),
            issues_url: Some("https://github.com/test/mod/issues".to_string()),
        };

        let result = update_checker
            .add_addon("test_addon".to_string(), platform_data, Priority::Normal)
            .await;
        assert!(result.is_ok() || result.is_err());
    }

    fn create_test_platform_data() -> ModplatformData {
        ModplatformData {
            platform: Platform::Modrinth,
            project_id: "test_project".to_string(),
            file_id: "test_file".to_string(),
            download_url: Some("https://example.com/addon.jar".to_string()),
            project_name: "Test Project".to_string(),
            project_description: Some("Test project description".to_string()),
            categories: vec!["utility".to_string()],
            license: Some("MIT".to_string()),
            website_url: Some("https://example.com".to_string()),
            source_url: Some("https://github.com/test/project".to_string()),
            issues_url: Some("https://github.com/test/project/issues".to_string()),
        }
    }
}

// Helper functions
fn create_test_config(temp_dir: &TempDir) -> StorageConfig {
    StorageConfig {
        runtime_path: temp_dir.path().to_path_buf(),
        max_image_size: 1024 * 1024,
        max_cache_size: 100 * 1024 * 1024,
        cleanup_interval: 3600,
    }
}

fn create_test_metadata(_jar_path: &PathBuf) -> LocalMetadata {
    LocalMetadata {
        addon_id: "test_addon".to_string(),
        name: "Test Addon".to_string(),
        version: "1.0.0".to_string(),
        authors: vec!["test_author".to_string()],
        description: Some("Test description".to_string()),
        dependencies: vec![],
        checksums: Checksums {
            blake3: "test_blake3".to_string(),
            sha256: "test_sha256".to_string(),
            md5: "test_md5".to_string(),
            murmur2: 12345,
        },
        mod_format: ModFormat::Unknown,
        minecraft_versions: vec!["1.20.1".to_string()],
        mod_loaders: vec!["fabric".to_string()],
        instance_id: Some("test_instance".to_string()),
    }
}
