use sqlx::SqlitePool;

pub mod account;
pub mod active_downloads;
pub mod app_configuration;
pub mod assets_meta_cache;
pub mod curseforge_mod_cache;
pub mod curseforge_mod_image_cache;
pub mod curseforge_modpack_cache;
pub mod curseforge_modpack_image_cache;
pub mod http_cache;
pub mod instance;
pub mod instance_group;
pub mod java;
pub mod java_profile;
pub mod local_mod_image_cache;
pub mod lwjgl_meta_cache;
pub mod mod_file_cache;
pub mod mod_metadata;
pub mod modrinth_mod_cache;
pub mod modrinth_mod_image_cache;
pub mod modrinth_modpack_cache;
pub mod modrinth_modpack_image_cache;
pub mod partial_version_info_cache;
pub mod skin;
pub mod version_info_cache;

pub struct Repo {
    pub account: account::AccountRepository,
    pub active_downloads: active_downloads::ActiveDownloadsRepository,
    pub app_configuration: app_configuration::AppConfigurationRepository,
    pub assets_meta_cache: assets_meta_cache::AssetsMetaCacheRepository,
    pub curseforge_mod_cache: curseforge_mod_cache::CurseForgeModCacheRepository,
    pub curseforge_mod_image_cache: curseforge_mod_image_cache::CurseForgeModImageCacheRepository,
    pub curseforge_modpack_cache: curseforge_modpack_cache::CurseForgeModpackCacheRepository,
    pub curseforge_modpack_image_cache:
        curseforge_modpack_image_cache::CurseForgeModpackImageCacheRepository,
    pub http_cache: http_cache::HTTPCacheRepository,
    pub instance: instance::InstanceRepository,
    pub instance_group: instance_group::InstanceGroupRepository,
    pub java: java::JavaRepository,
    pub java_profile: java_profile::JavaProfileRepository,
    pub local_mod_image_cache: local_mod_image_cache::LocalModImageCacheRepository,
    pub lwjgl_meta_cache: lwjgl_meta_cache::LwjglMetaCacheRepository,
    pub mod_file_cache: mod_file_cache::ModFileCacheRepository,
    pub mod_metadata: mod_metadata::ModMetadataRepository,
    pub modrinth_mod_cache: modrinth_mod_cache::ModrinthModCacheRepository,
    pub modrinth_mod_image_cache: modrinth_mod_image_cache::ModrinthModImageCacheRepository,
    pub modrinth_modpack_cache: modrinth_modpack_cache::ModrinthModpackCacheRepository,
    pub modrinth_modpack_image_cache:
        modrinth_modpack_image_cache::ModrinthModpackImageCacheRepository,
    pub partial_version_info_cache: partial_version_info_cache::PartialVersionInfoCacheRepository,
    pub skin: skin::SkinRepository,
    pub version_info_cache: version_info_cache::VersionInfoCacheRepository,
}

impl Repo {
    pub fn new(pool: SqlitePool) -> Self {
        Repo {
            account: account::AccountRepository::new(pool.clone()),
            active_downloads: active_downloads::ActiveDownloadsRepository::new(pool.clone()),
            app_configuration: app_configuration::AppConfigurationRepository::new(pool.clone()),
            assets_meta_cache: assets_meta_cache::AssetsMetaCacheRepository::new(pool.clone()),
            curseforge_mod_cache: curseforge_mod_cache::CurseForgeModCacheRepository::new(
                pool.clone(),
            ),
            curseforge_mod_image_cache:
                curseforge_mod_image_cache::CurseForgeModImageCacheRepository::new(pool.clone()),
            curseforge_modpack_cache:
                curseforge_modpack_cache::CurseForgeModpackCacheRepository::new(pool.clone()),
            curseforge_modpack_image_cache:
                curseforge_modpack_image_cache::CurseForgeModpackImageCacheRepository::new(
                    pool.clone(),
                ),
            http_cache: http_cache::HTTPCacheRepository::new(pool.clone()),
            instance: instance::InstanceRepository::new(pool.clone()),
            instance_group: instance_group::InstanceGroupRepository::new(pool.clone()),
            java: java::JavaRepository::new(pool.clone()),
            java_profile: java_profile::JavaProfileRepository::new(pool.clone()),
            local_mod_image_cache: local_mod_image_cache::LocalModImageCacheRepository::new(
                pool.clone(),
            ),
            lwjgl_meta_cache: lwjgl_meta_cache::LwjglMetaCacheRepository::new(pool.clone()),
            mod_file_cache: mod_file_cache::ModFileCacheRepository::new(pool.clone()),
            mod_metadata: mod_metadata::ModMetadataRepository::new(pool.clone()),
            modrinth_mod_cache: modrinth_mod_cache::ModrinthModCacheRepository::new(pool.clone()),
            modrinth_mod_image_cache:
                modrinth_mod_image_cache::ModrinthModImageCacheRepository::new(pool.clone()),
            modrinth_modpack_cache: modrinth_modpack_cache::ModrinthModpackCacheRepository::new(
                pool.clone(),
            ),
            modrinth_modpack_image_cache:
                modrinth_modpack_image_cache::ModrinthModpackImageCacheRepository::new(pool.clone()),
            partial_version_info_cache:
                partial_version_info_cache::PartialVersionInfoCacheRepository::new(pool.clone()),
            skin: skin::SkinRepository::new(pool.clone()),
            version_info_cache: version_info_cache::VersionInfoCacheRepository::new(pool.clone()),
        }
    }
}
