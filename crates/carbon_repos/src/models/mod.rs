//! Database model structs.
//!
//! These structs map directly to the SQLite database tables and provide
//! type-safe representations of database rows.

pub mod settings;
pub mod account;
pub mod java;
pub mod instance;
pub mod cache;
pub mod metadata;
pub mod modpack;

// Re-export commonly used types
pub use settings::AppConfiguration;
pub use account::{Account, Skin};
pub use java::{Java, JavaProfile, JavaProfileWithPath};
pub use instance::{Instance, InstanceGroup};
pub use cache::{
    HTTPCache, ActiveDownload, VersionInfoCache, PartialVersionInfoCache,
    LwjglMetaCache, AssetsMetaCache,
};
pub use metadata::{
    ModFileCache, ModMetadata, CurseForgeModCache, ModrinthModCache,
    LocalModImageCache, CurseForgeModImageCache, ModrinthModImageCache,
    ModFileCacheWithMetadata, ModFileCacheWithMetadataAndImages,
    ModFileCacheWithCurseforge, ModFileCacheWithModrinth,
};
pub use modpack::{
    CurseForgeModpackCache, ModrinthModpackCache,
    CurseForgeModpackImageCache, ModrinthModpackImageCache,
};
