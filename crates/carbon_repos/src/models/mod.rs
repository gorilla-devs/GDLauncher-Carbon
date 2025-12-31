//! Database model structs.
//!
//! These structs map directly to the SQLite database tables and provide
//! type-safe representations of database rows.

pub mod account;
pub mod cache;
pub mod instance;
pub mod java;
pub mod metadata;
pub mod modpack;
pub mod settings;

// Re-export commonly used types
pub use account::{Account, Skin};
pub use cache::{
    ActiveDownload, AssetsMetaCache, HTTPCache, LwjglMetaCache, PartialVersionInfoCache,
    VersionInfoCache,
};
pub use instance::{Instance, InstanceGroup};
pub use java::{Java, JavaProfile, JavaProfileWithPath};
pub use metadata::{
    CurseForgeModCache, CurseForgeModImageCache, LocalModImageCache, ModFileCache,
    ModFileCacheWithCurseforge, ModFileCacheWithMetadata, ModFileCacheWithMetadataAndImages,
    ModFileCacheWithModrinth, ModMetadata, ModrinthModCache, ModrinthModImageCache,
};
pub use modpack::{
    CurseForgeModpackCache, CurseForgeModpackImageCache, ModpackCacheEntry, ModrinthModpackCache,
    ModrinthModpackImageCache,
};
pub use settings::AppConfiguration;
