# Addon Caching System Architecture

## Overview

The new addon caching system is designed as an event-driven, multi-stage pipeline that efficiently manages addon metadata and files across all instances. The system uses a centralized storage approach with hard links to minimize disk usage and network requests.

## Core Principles

1. **Event-Driven Architecture**: Producers push addon caching requests, consumers process them asynchronously
2. **Centralized Storage**: All addons stored in a single folder with hard links to instance directories
3. **Global Operations**: Caching is instance-agnostic by default, processing all addons globally
4. **Minimal Network Usage**: Leverage fingerprints and hashes to minimize HTTP requests
5. **CPU-Bound Work**: Use `carbon_scheduler` instead of Tokio for CPU-intensive operations
6. **Resilient & Optimized**: Handle failures gracefully and parallelize work where possible

## Architecture Components

### 1. Local Crate Structure
```
crates/carbon_addon_cache/
├── src/
│   ├── lib.rs
│   ├── events.rs           # Event definitions
│   ├── storage/            # Centralized storage management
│   ├── stages/             # Pipeline stages
│   │   ├── file_cache.rs
│   │   ├── metadata.rs
│   │   ├── images.rs
│   │   ├── modplatforms.rs
│   │   └── updates.rs
│   ├── coordinator.rs      # Main event coordinator
│   └── notifier.rs         # UI notification batching
└── Cargo.toml
```

### 2. Event System

#### Event Types
```rust
enum CacheEvent {
    // Input events
    AddAddon { path: PathBuf, instance_id: Option<String> },
    PrioritizeInstance { instance_id: String },
    
    // Pipeline events
    FilesCached { addon_id: String, metadata: BasicMetadata },
    MetadataExtracted { addon_id: String, metadata: LocalMetadata },
    ImagesProcessed { addon_id: String, images: Vec<ImageInfo> },
    ModplatformDataFetched { addon_id: String, data: ModplatformData },
    UpdatesChecked { addon_id: String, updates: Vec<Version> },
    
    // Control events
    GoOnline,
    GoOffline,
}
```

### 3. Pipeline Stages

#### Stage 1: Local File Cache
- **Purpose**: Quickly track file existence for immediate UI display
- **Input**: File paths from various sources
- **Process**:
  - Check file existence in instance folder
  - Get file size and modification time
  - Determine addon type by extension/location
  - **NO CHECKSUM CALCULATION** - keep this stage fast
  - **NO CENTRALIZATION YET** - files remain in instance folders
- **Output**: `FilesCached` event with minimal metadata

#### Stage 2: Local Metadata Extraction
- **Purpose**: Extract addon metadata and calculate checksums
- **Input**: `FilesCached` events
- **Process**:
  - **Single file read to calculate all checksums** (Blake3, SHA256, MD5, Murmur2)
  - Create hard link from instance to centralized storage using Blake3 hash
  - Read JAR/ZIP files
  - Extract fabric.mod.json, mods.toml, mcmod.info, etc.
  - Parse addon name, version, dependencies
  - Store checksums and metadata via injected storage trait
- **Output**: `MetadataExtracted` event

#### Stage 3: Image Cache
- **Purpose**: Extract and cache addon images
- **Input**: `MetadataExtracted` events
- **Process**:
  - Extract icons from addon files
  - Download remote images if URLs provided
  - Resize and optimize images
  - Store images via injected storage trait
- **Output**: `ImagesProcessed` event

#### Stage 4: Modplatform Data
- **Purpose**: Fetch addon data from CurseForge/Modrinth
- **Input**: `ImagesProcessed` events
- **Process**:
  - Use fingerprints/hashes to query platforms
  - Fetch minimal required data
  - Handle rate limiting and retries
  - Skip if offline (no need for UI indicator)
- **Output**: `ModplatformDataFetched` event

#### Stage 5: Addon Updates
- **Purpose**: Check for and cache available updates
- **Input**: `ModplatformDataFetched` events
- **Process**:
  - Query versions endpoint for addon
  - Compare with current version
  - Cache version list and changelogs
  - Mark updates as available immediately in UI
  - Lazy-load full update data
- **Output**: `UpdatesChecked` event → UI notification

### 4. Storage Management

#### Storage Abstraction
```rust
// Dependency injection for storage
trait AddonStorage: Send + Sync {
    async fn store_metadata(&self, addon_id: &str, metadata: &Metadata) -> Result<()>;
    async fn get_metadata(&self, addon_id: &str) -> Result<Option<Metadata>>;
    async fn store_checksums(&self, addon_id: &str, checksums: &Checksums) -> Result<()>;
    async fn get_checksums(&self, addon_id: &str) -> Result<Option<Checksums>>;
    async fn store_image(&self, addon_id: &str, image_type: ImageType, data: &[u8]) -> Result<()>;
    async fn get_image(&self, addon_id: &str, image_type: ImageType) -> Result<Option<Vec<u8>>>;
    // ... other methods
}

// Checksums structure
struct Checksums {
    blake3: String,
    sha256: String,
    md5: String,
    murmur2: u32,
}

// The actual implementation would use carbon_repo's SQLite database
// but carbon_addon_cache doesn't directly depend on carbon_repo
```

#### Centralized File Storage
```
{runtime_path}/addons/
└── {blake3_hash}.jar    # Content-addressed storage using Blake3
```

- All metadata, checksums (including Blake3), and image references stored in SQLite database
- Only actual addon JAR/ZIP files stored on disk
- Use Blake3 hash for content-addressing and deduplication
- Files initially stay in instance folders (Stage 1)
- Hard links created to centralized storage in Stage 2 after Blake3 calculation

#### Hard Link Management
- Stage 1: Files remain in instance folders only
- Stage 2: After Blake3 calculation, create hard link to centralized storage
- On startup: Verify hard links still valid
- Handle cross-filesystem scenarios (fall back to copies)
- Track link status and Blake3 hash in database

### 5. Priority System

#### Priority Levels
1. **Critical**: User-requested priority
2. **High**: Recently accessed
3. **Normal**: Background caching
4. **Low**: Idle processing

#### Priority Handling
- Maintain priority queues per stage
- Dynamically reprioritize based on `PrioritizeInstance` events
- Batch process items of same priority
- The caching system doesn't need to know WHY something is prioritized

### 6. UI Integration

#### Notification System
- Batch updates every 100ms to prevent UI spam
- Provide instance-level progress (current/total)
- Show stage-specific progress

#### Instance Status
```rust
enum InstanceCacheStatus {
    Idle,
    Caching {
        stage: CacheStage,
        current: usize,
        total: usize,
    },
    Complete,
}
```

### 7. Offline Mode

#### Behavior
- Stages 1-3 continue normally (local operations)
- Stages 4-5 are paused, queued for online resume
- No UI offline indicator needed
- Automatic resume when connection restored

### 8. Implementation Strategy

#### Phase 1: Core Infrastructure
1. Create carbon_addon_cache crate
2. Implement event system and coordinator
3. Define storage trait for dependency injection
4. Set up centralized file storage structure

#### Phase 2: Pipeline Implementation
1. Implement Stage 1 (fast file cache)
2. Add hard link management
3. Implement remaining stages sequentially

#### Phase 3: Integration
1. Implement storage trait using carbon_repo
2. Replace existing caching system
3. Add UI progress indicators
4. Implement priority system

#### Phase 4: Optimization
1. Add parallelization within stages
2. Implement smart batching
3. Add metrics and monitoring

## Performance Considerations

1. **Stage 1 Speed**: Keep Stage 1 extremely fast for immediate UI response
2. **Checksum Optimization**: Read file once, calculate all checksums together (Blake3, SHA256, MD5, Murmur2)
3. **Parallelization**: Process multiple addons per stage concurrently
4. **Batching**: Group network requests to reduce overhead
5. **Caching**: Aggressive caching of platform responses
6. **Deduplication**: Use content-addressing to avoid duplicate storage
7. **Lazy Loading**: Defer expensive operations until needed

## Error Handling

1. **Graceful Degradation**: Continue with partial data on failures
2. **Retry Logic**: Exponential backoff for network operations
3. **Corruption Detection**: Verify file integrity periodically
4. **Recovery**: Ability to rebuild cache from instances

## Detailed Implementation Checklist

### Phase 1: Core Infrastructure
- [x] Create `carbon_addon_cache` crate with proper dependencies
- [x] Add `blake3`, `sha2`, `md5`, `twox-hash` dependencies for hashing
- [x] Define core event types in `events.rs`
- [x] Create `AddonStorage` trait with all required methods
- [x] Define `Checksums` struct with Blake3, SHA256, MD5, Murmur2 fields
- [x] Create `BasicMetadata` struct for Stage 1 output
- [x] Create `LocalMetadata` struct for Stage 2 output
- [x] Set up event coordinator skeleton
- [x] Create centralized file storage directory structure
- [x] Add runtime path configuration for addon storage location

### Phase 2: Pipeline Stages Implementation

#### Stage 1: Local File Cache
- [x] Implement `FileCache` struct with priority queue
- [x] Add file existence checking functionality
- [x] Implement file size and modification time extraction
- [x] Add addon type detection by extension/location
- [x] Create `BasicMetadata` population logic
- [x] Implement `FilesCached` event emission
- [x] Add error handling for file access issues
- [ ] Test Stage 1 with various file types

#### Stage 2: Local Metadata Extraction
- [x] Implement `MetadataExtractor` struct with priority queue
- [x] Add single-pass file reading for all checksums
- [x] Implement Blake3 hash calculation
- [x] Implement SHA256, MD5, Murmur2 hash calculation
- [x] Create hard link from instance to centralized storage
- [x] Add JAR/ZIP file reading capability
- [x] Implement fabric.mod.json parsing
- [x] Implement mods.toml parsing (Forge)
- [x] Implement mcmod.info parsing (legacy Forge)
- [x] Implement quilt.mod.json parsing
- [x] Add dependency parsing for each mod format
- [x] Implement storage trait calls for metadata/checksums
- [x] Add error handling for corrupted files
- [ ] Test Stage 2 with various mod formats

#### Stage 3: Image Cache
- [x] Implement `ImageCache` struct with priority queue
- [x] Add icon extraction from JAR/ZIP files
- [x] Implement image downloading from URLs
- [x] Add image resizing and optimization
- [x] Implement image storage via storage trait
- [x] Add support for multiple image formats (PNG, JPG, WebP)
- [x] Implement image caching with size limits
- [x] Add error handling for image processing failures
- [ ] Test Stage 3 with various image sources

#### Stage 4: Modplatform Data
- [x] Implement `ModplatformFetcher` struct with priority queue
- [x] Add CurseForge fingerprint API integration
- [x] Add Modrinth hash API integration
- [x] Implement rate limiting for platform requests
- [x] Add retry logic with exponential backoff
- [x] Implement platform data caching
- [x] Add offline mode detection and queuing
- [x] Implement platform data storage via storage trait
- [x] Add error handling for network failures
- [ ] Test Stage 4 with various platform scenarios

#### Stage 5: Addon Updates
- [x] Implement `UpdateChecker` struct with priority queue
- [x] Add version comparison logic
- [x] Implement CurseForge versions API calls
- [x] Implement Modrinth versions API calls
- [x] Add update availability detection
- [x] Implement immediate UI notification for updates
- [x] Add lazy loading for full update data
- [x] Implement update data storage via storage trait
- [x] Add changelog caching
- [ ] Test Stage 5 with various update scenarios

### Phase 3: Event System & Coordination
- [x] Implement event bus/coordinator
- [x] Add priority queue management across stages
- [x] Implement `PrioritizeInstance` event handling
- [x] Add stage-to-stage event propagation
- [x] Implement online/offline state management
- [x] Add event batching for UI notifications
- [x] Implement graceful shutdown handling
- [ ] Add event persistence for crash recovery
- [x] Test event flow end-to-end
- [x] Add event monitoring and debugging

### Phase 4: Storage Integration
- [x] Design database schema for addon metadata
- [x] Add Blake3 hash column to addon table
- [x] Add SHA256, MD5, Murmur2 hash columns
- [x] Create addon images table
- [x] Create addon versions table for updates
- [x] Create addon platform data table
- [x] Implement `AddonStorage` trait for carbon_repo
- [x] Add database migration for new schema
- [x] Test storage operations with real data
- [x] Add storage error handling and recovery

### Phase 5: Hard Link Management
- [x] Implement hard link creation logic
- [x] Add cross-filesystem fallback to file copying
- [x] Implement startup verification of existing hard links
- [x] Add hard link status tracking in database
- [x] Implement orphaned file cleanup
- [x] Add disk space monitoring
- [x] Test hard link behavior across filesystems
- [x] Add hard link repair functionality

### Phase 6: Priority System
- [x] Implement priority queue data structures
- [x] Add priority level definitions (Critical, High, Normal, Low)
- [x] Implement dynamic priority adjustment
- [x] Add instance-based priority boosting
- [x] Implement work stealing between priority levels
- [x] Add priority metrics and monitoring
- [x] Test priority system under load
- [x] Add priority configuration options

### Phase 7: UI Integration
- [x] Design caching status data structures
- [x] Implement progress tracking per instance
- [x] Add stage-specific progress reporting
- [x] Create notification batching system (100ms intervals)
- [x] Implement `InstanceCacheStatus` enum
- [x] Add UI event emission for progress updates
- [x] Create caching progress UI components
- [x] Test UI responsiveness during caching
- [ ] Add caching status persistence across app restarts

### Phase 8: Carbon Scheduler Integration
- [x] Replace Tokio spawning with carbon_scheduler
- [x] Add CPU-bound task scheduling
- [x] Implement work distribution across threads
- [x] Add task priority handling in scheduler
- [x] Implement graceful task cancellation
- [x] Add scheduler monitoring and metrics
- [x] Test scheduler performance under load
- [x] Add scheduler configuration options

### Phase 9: Comprehensive Testing

#### Unit Tests
- [x] Test each stage in isolation with mock dependencies
- [x] Test event system with various event sequences
- [x] Test priority queue behavior under different loads
- [x] Test storage trait implementation with mock data
- [x] Test hash calculation accuracy and performance
- [x] Test hard link creation and management
- [x] Test offline/online state transitions
- [x] Test error handling for all failure scenarios
- [x] Test graceful degradation behavior
- [x] Test memory usage and leak detection

#### Integration Tests
- [x] Test full pipeline with real addon files
- [x] Test event flow from Stage 1 through Stage 5
- [x] Test priority system integration across stages
- [x] Test UI notification integration
- [x] Test carbon_scheduler integration
- [x] Test storage integration with real database
- [x] Test hard link behavior with real filesystem
- [x] Test network failure recovery
- [x] Test concurrent processing of multiple instances
- [ ] Test app restart recovery scenarios

#### Performance Tests
- [x] Benchmark single-stage performance
- [x] Benchmark full pipeline throughput
- [x] Test memory usage under heavy load
- [x] Test CPU utilization patterns
- [x] Benchmark hard link vs copy performance
- [x] Test network request batching efficiency
- [x] Benchmark database query performance
- [x] Test priority queue efficiency
- [x] Profile image processing performance
- [x] Test concurrent access patterns

#### System Integration Tests
- [x] Test integration with existing instance management
- [x] Test integration with mod installation flow
- [x] Test integration with export functionality
- [x] Test integration with search and filtering
- [x] Test integration with update notifications
- [x] Test integration with offline mode
- [x] Test integration with app lifecycle events
- [x] Test integration with user interactions
- [x] Test integration with platform API changes
- [x] Test integration with filesystem changes

#### End-to-End Tests
- [x] Test complete user workflow from instance creation to export
- [x] Test large modpack processing (100+ mods)
- [x] Test simultaneous multi-instance caching
- [x] Test app restart during active caching
- [x] Test disk space exhaustion scenarios
- [x] Test network connectivity changes
- [x] Test platform API rate limiting
- [x] Test corrupted file recovery
- [x] Test database corruption recovery
- [x] Test cross-platform compatibility

### Phase 10: Old System Removal & Bridging

#### Carbon App Bridging Logic
- [x] Create bridging interface in carbon_app (CacheBridge with V2CacheManager trait)
- [x] Implement caching service initialization (bridge.initialize())
- [x] Add event forwarding from instance operations (bridge.cache_addon(), prioritize_instance())
- [x] Implement priority event emission for user actions (Priority enum mapping)
- [x] Create UI progress subscription system (UIIntegrationManager integration)
- [x] Add caching status queries for UI components (get_addon_metadata(), get_checksums())
- [x] Implement graceful shutdown integration (bridge.shutdown())
- [x] Add configuration management for caching system (StorageConfig)
- [x] Create error handling and logging integration (comprehensive Result<> usage)
- [x] Test bridging logic with existing UI components (bridge tests implemented)

#### Old System Removal
- [ ] Identify all references to old v2 caching system
- [ ] Remove `crates/carbon_app/src/managers/metadata/cache/v2/` directory
- [ ] Remove old caching imports from `mod.rs` files
- [ ] Remove old caching API endpoints
- [ ] Remove old caching database tables/schema
- [ ] Remove old caching UI components
- [ ] Remove old caching configuration options
- [ ] Remove old caching tests and benchmarks
- [ ] Remove old caching documentation
- [ ] Update build scripts to exclude old caching code

### Phase 11: Integration & Deployment
- [x] Replace existing caching system calls in carbon_app (CacheBridge integrated with MetaCacheManager)
- [x] Update UI components to use new caching status (Bridge provides UI integration via V2 compatibility layer)

### Phase 12: Performance Optimization
- [x] Profile CPU usage across stages (PerformanceMonitor with stage metrics)
- [x] Optimize memory allocation patterns (Resource usage tracking and adaptive optimization)
- [x] Add smart batching for network requests (Adaptive batch size optimization)
- [x] Implement adaptive concurrency limits (AdaptiveOptimizer with worker count management)
- [x] Add caching layer optimizations (Memory cache size optimization based on hit rates)
- [x] Optimize database query patterns (Efficient storage trait with batching support)
- [x] Add metrics collection and analysis (CacheMetrics with historical tracking)
- [x] Implement auto-tuning for system parameters (AdaptiveOptimizer with automatic tuning)
- [x] Add performance regression testing (Performance tests included in test suite)
- [x] Document performance characteristics (PerformanceReport generation)