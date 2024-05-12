-- Add migration script here
-- Renaming tables to snake_case
ALTER TABLE
  AppConfiguration RENAME TO temp_app_configuration;

ALTER TABLE
  temp_app_configuration RENAME TO app_configuration;

ALTER TABLE
  Java RENAME TO temp_java;

ALTER TABLE
  temp_java RENAME TO java;

ALTER TABLE
  JavaProfile RENAME TO temp_java_profile;

ALTER TABLE
  temp_java_profile RENAME TO java_profile;

ALTER TABLE
  Account RENAME TO temp_account;

ALTER TABLE
  temp_account RENAME TO account;

ALTER TABLE
  Skin RENAME TO temp_skin;

ALTER TABLE
  temp_skin RENAME TO skin;

ALTER TABLE
  HTTPCache RENAME TO temp_http_cache;

ALTER TABLE
  temp_http_cache RENAME TO http_cache;

ALTER TABLE
  ActiveDownloads RENAME TO temp_active_downloads;

ALTER TABLE
  temp_active_downloads RENAME TO active_downloads;

ALTER TABLE
  Instance RENAME TO temp_instance;

ALTER TABLE
  temp_instance RENAME TO instance;

ALTER TABLE
  VersionInfoCache RENAME TO temp_version_info_cache;

ALTER TABLE
  temp_version_info_cache RENAME TO version_info_cache;

ALTER TABLE
  PartialVersionInfoCache RENAME TO temp_partial_version_info_cache;

ALTER TABLE
  temp_partial_version_info_cache RENAME TO partial_version_info_cache;

ALTER TABLE
  LwjglMetaCache RENAME TO temp_lwjgl_meta_cache;

ALTER TABLE
  temp_lwjgl_meta_cache RENAME TO lwjgl_meta_cache;

ALTER TABLE
  AssetsMetaCache RENAME TO temp_assets_meta_cache;

ALTER TABLE
  temp_assets_meta_cache RENAME TO assets_meta_cache;

ALTER TABLE
  InstanceGroup RENAME TO temp_instance_group;

ALTER TABLE
  temp_instance_group RENAME TO instance_group;

ALTER TABLE
  ModFileCache RENAME TO temp_mod_file_cache;

ALTER TABLE
  temp_mod_file_cache RENAME TO mod_file_cache;

ALTER TABLE
  ModMetadata RENAME TO temp_mod_metadata;

ALTER TABLE
  temp_mod_metadata RENAME TO mod_metadata;

ALTER TABLE
  CurseForgeModCache RENAME TO temp_curseforge_mod_cache;

ALTER TABLE
  temp_curseforge_mod_cache RENAME TO curseforge_mod_cache;

ALTER TABLE
  ModrinthModCache RENAME TO temp_modrinth_mod_cache;

ALTER TABLE
  temp_modrinth_mod_cache RENAME TO modrinth_mod_cache;

ALTER TABLE
  LocalModImageCache RENAME TO temp_local_mod_image_cache;

ALTER TABLE
  temp_local_mod_image_cache RENAME TO local_mod_image_cache;

ALTER TABLE
  CurseForgeModImageCache RENAME TO temp_curseforge_mod_image_cache;

ALTER TABLE
  temp_curseforge_mod_image_cache RENAME TO curseforge_mod_image_cache;

ALTER TABLE
  ModrinthModImageCache RENAME TO temp_modrinth_mod_image_cache;

ALTER TABLE
  temp_modrinth_mod_image_cache RENAME TO modrinth_mod_image_cache;

ALTER TABLE
  CurseForgeModpackCache RENAME TO temp_curseforge_modpack_cache;

ALTER TABLE
  temp_curseforge_modpack_cache RENAME TO curseforge_modpack_cache;

ALTER TABLE
  ModrinthModpackCache RENAME TO temp_modrinth_modpack_cache;

ALTER TABLE
  temp_modrinth_modpack_cache RENAME TO modrinth_modpack_cache;

ALTER TABLE
  CurseForgeModpackImageCache RENAME TO temp_curseforge_modpack_image_cache;

ALTER TABLE
  temp_curseforge_modpack_image_cache RENAME TO curseforge_modpack_image_cache;

ALTER TABLE
  ModrinthModpackImageCache RENAME TO temp_modrinth_modpack_image_cache;

ALTER TABLE
  temp_modrinth_modpack_image_cache RENAME TO modrinth_modpack_image_cache;

-- Renaming columns in app_configuration
ALTER TABLE
  app_configuration RENAME COLUMN reducedMotion TO reduced_motion;

ALTER TABLE
  app_configuration RENAME COLUMN discordIntegration TO discord_integration;

ALTER TABLE
  app_configuration RENAME COLUMN releaseChannel TO release_channel;

ALTER TABLE
  app_configuration RENAME COLUMN lastAppVersion TO last_app_version;

ALTER TABLE
  app_configuration RENAME COLUMN activeAccountUuid TO active_account_uuid;

ALTER TABLE
  app_configuration RENAME COLUMN concurrentDownloads TO concurrent_downloads;

ALTER TABLE
  app_configuration RENAME COLUMN downloadDependencies TO download_dependencies;

ALTER TABLE
  app_configuration RENAME COLUMN instancesTileSize TO instances_tile_size;

ALTER TABLE
  app_configuration RENAME COLUMN instancesGroupBy TO instances_group_by;

ALTER TABLE
  app_configuration RENAME COLUMN instancesGroupByAsc TO instances_group_by_asc;

ALTER TABLE
  app_configuration RENAME COLUMN instancesSortBy TO instances_sort_by;

ALTER TABLE
  app_configuration RENAME COLUMN instancesSortByAsc TO instances_sort_by_asc;

ALTER TABLE
  app_configuration RENAME COLUMN showNews TO show_news;

ALTER TABLE
  app_configuration RENAME COLUMN deletionThroughRecycleBin TO deletion_through_recycle_bin;

ALTER TABLE
  app_configuration RENAME COLUMN gameResolution TO game_resolution;

ALTER TABLE
  app_configuration RENAME COLUMN launcherActionOnGameLaunch TO launcher_action_on_game_launch;

ALTER TABLE
  app_configuration RENAME COLUMN showAppCloseWarning TO show_app_close_warning;

ALTER TABLE
  app_configuration RENAME COLUMN javaCustomArgs TO java_custom_args;

ALTER TABLE
  app_configuration RENAME COLUMN defaultInstanceGroup TO default_instance_group;

ALTER TABLE
  app_configuration RENAME COLUMN preLaunchHook TO pre_launch_hook;

ALTER TABLE
  app_configuration RENAME COLUMN wrapperCommand TO wrapper_command;

ALTER TABLE
  app_configuration RENAME COLUMN postExitHook TO post_exit_hook;

ALTER TABLE
  app_configuration RENAME COLUMN isFirstLaunch TO is_first_launch;

ALTER TABLE
  app_configuration RENAME COLUMN autoManageJavaSystemProfiles TO auto_manage_java_system_profiles;

ALTER TABLE
  app_configuration RENAME COLUMN modPlatformBlacklist TO mod_platform_blacklist;

ALTER TABLE
  app_configuration RENAME COLUMN modChannels TO mod_channels;

ALTER TABLE
  app_configuration RENAME COLUMN randomUserUuid TO random_user_uuid;

ALTER TABLE
  app_configuration RENAME COLUMN secret TO secret;

ALTER TABLE
  app_configuration RENAME COLUMN termsAndPrivacyAccepted TO terms_and_privacy_accepted;

ALTER TABLE
  app_configuration RENAME COLUMN termsAndPrivacyAcceptedChecksum TO terms_and_privacy_accepted_checksum;

ALTER TABLE
  app_configuration RENAME COLUMN metricsEnabled TO metrics_enabled;

ALTER TABLE
  app_configuration RENAME COLUMN metricsEnabledLastUpdate TO metrics_enabled_last_update;

-- Renaming columns in java
ALTER TABLE
  java RENAME COLUMN fullVersion TO full_version;

ALTER TABLE
  java RENAME COLUMN isValid TO is_valid;

-- Renaming columns in java_profile
ALTER TABLE
  java_profile RENAME COLUMN isSystemProfile TO is_system_profile;

ALTER TABLE
  java_profile RENAME COLUMN javaId TO java_id;

-- Renaming columns in account
ALTER TABLE
  account RENAME COLUMN accessToken TO access_token;

ALTER TABLE
  account RENAME COLUMN tokenExpires TO token_expires;

ALTER TABLE
  account RENAME COLUMN msRefreshToken TO ms_refresh_token;

ALTER TABLE
  account RENAME COLUMN idToken TO id_token;

ALTER TABLE
  account RENAME COLUMN lastUsed TO last_used;

ALTER TABLE
  account RENAME COLUMN skinId TO skin_id;

-- Renaming columns in skin
-- No renaming needed
-- Renaming columns in http_cache
ALTER TABLE
  http_cache RENAME COLUMN status_code TO status_code;

ALTER TABLE
  http_cache RENAME COLUMN expiresAt TO expires_at;

ALTER TABLE
  http_cache RENAME COLUMN lastModified TO last_modified;

-- Renaming columns in active_downloads
-- No renaming needed
-- Renaming columns in instance
ALTER TABLE
  instance RENAME COLUMN shortpath TO shortpath;

ALTER TABLE
  instance RENAME COLUMN favorite TO favorite;

ALTER TABLE
  instance RENAME COLUMN hasPackUpdate TO has_pack_update;

ALTER TABLE
  instance RENAME COLUMN groupId TO group_id;

-- Renaming columns in version_info_cache
ALTER TABLE
  version_info_cache RENAME COLUMN lastUpdatedAt TO last_updated_at;

ALTER TABLE
  version_info_cache RENAME COLUMN versionInfo TO version_info;

-- Renaming columns in partial_version_info_cache
ALTER TABLE
  partial_version_info_cache RENAME COLUMN lastUpdatedAt TO last_updated_at;

ALTER TABLE
  partial_version_info_cache RENAME COLUMN partialVersionInfo TO partial_version_info;

-- Renaming columns in lwjgl_meta_cache
ALTER TABLE
  lwjgl_meta_cache RENAME COLUMN lastUpdatedAt TO last_updated_at;

ALTER TABLE
  lwjgl_meta_cache RENAME COLUMN lwjgl TO lwjgl;

-- Renaming columns in assets_meta_cache
ALTER TABLE
  assets_meta_cache RENAME COLUMN lastUpdatedAt TO last_updated_at;

ALTER TABLE
  assets_meta_cache RENAME COLUMN assetsIndex TO assets_index;

-- Renaming columns in instance_group
ALTER TABLE
  instance_group RENAME COLUMN groupIndex TO group_index;

-- Renaming columns in mod_file_cache
ALTER TABLE
  mod_file_cache RENAME COLUMN lastUpdatedAt TO last_updated_at;

ALTER TABLE
  mod_file_cache RENAME COLUMN instanceId TO instance_id;

ALTER TABLE
  mod_file_cache RENAME COLUMN filename TO filename;

ALTER TABLE
  mod_file_cache RENAME COLUMN filesize TO filesize;

ALTER TABLE
  mod_file_cache RENAME COLUMN enabled TO enabled;

ALTER TABLE
  mod_file_cache RENAME COLUMN metadataId TO metadata_id;

-- Renaming columns in mod_metadata
ALTER TABLE
  mod_metadata RENAME COLUMN lastUpdatedAt TO last_updated_at;

ALTER TABLE
  mod_metadata RENAME COLUMN murmur2 TO murmur2;

ALTER TABLE
  mod_metadata RENAME COLUMN sha512 TO sha512;

ALTER TABLE
  mod_metadata RENAME COLUMN sha1 TO sha1;

ALTER TABLE
  mod_metadata RENAME COLUMN name TO name;

ALTER TABLE
  mod_metadata RENAME COLUMN modid TO modid;

ALTER TABLE
  mod_metadata RENAME COLUMN version TO version;

ALTER TABLE
  mod_metadata RENAME COLUMN description TO description;

ALTER TABLE
  mod_metadata RENAME COLUMN authors TO authors;

ALTER TABLE
  mod_metadata RENAME COLUMN modloaders TO modloaders;

-- Renaming columns in curseforge_mod_cache
ALTER TABLE
  curseforge_mod_cache RENAME COLUMN metadataId TO metadata_id;

ALTER TABLE
  curseforge_mod_cache RENAME COLUMN murmur2 TO murmur2;

ALTER TABLE
  curseforge_mod_cache RENAME COLUMN projectId TO project_id;

ALTER TABLE
  curseforge_mod_cache RENAME COLUMN fileId TO file_id;

ALTER TABLE
  curseforge_mod_cache RENAME COLUMN name TO name;

ALTER TABLE
  curseforge_mod_cache RENAME COLUMN version TO version;

ALTER TABLE
  curseforge_mod_cache RENAME COLUMN urlslug TO urlslug;

ALTER TABLE
  curseforge_mod_cache RENAME COLUMN summary TO summary;

ALTER TABLE
  curseforge_mod_cache RENAME COLUMN authors TO authors;

ALTER TABLE
  curseforge_mod_cache RENAME COLUMN releaseType TO release_type;

ALTER TABLE
  curseforge_mod_cache RENAME COLUMN updatePaths TO update_paths;

ALTER TABLE
  curseforge_mod_cache RENAME COLUMN cachedAt TO cached_at;

-- Renaming columns in modrinth_mod_cache
ALTER TABLE
  modrinth_mod_cache RENAME COLUMN metadataId TO metadata_id;

ALTER TABLE
  modrinth_mod_cache RENAME COLUMN sha512 TO sha512;

ALTER TABLE
  modrinth_mod_cache RENAME COLUMN projectId TO project_id;

ALTER TABLE
  modrinth_mod_cache RENAME COLUMN versionId TO version_id;

ALTER TABLE
  modrinth_mod_cache RENAME COLUMN title TO title;

ALTER TABLE
  modrinth_mod_cache RENAME COLUMN version TO version;

ALTER TABLE
  modrinth_mod_cache RENAME COLUMN urlslug TO urlslug;

ALTER TABLE
  modrinth_mod_cache RENAME COLUMN description TO description;

ALTER TABLE
  modrinth_mod_cache RENAME COLUMN authors TO authors;

ALTER TABLE
  modrinth_mod_cache RENAME COLUMN releaseType TO release_type;

ALTER TABLE
  modrinth_mod_cache RENAME COLUMN updatePaths TO update_paths;

ALTER TABLE
  modrinth_mod_cache RENAME COLUMN filename TO filename;

ALTER TABLE
  modrinth_mod_cache RENAME COLUMN fileUrl TO file_url;

ALTER TABLE
  modrinth_mod_cache RENAME COLUMN cachedAt TO cached_at;

-- Renaming columns in local_mod_image_cache
ALTER TABLE
  local_mod_image_cache RENAME COLUMN metadataId TO metadata_id;

-- Renaming columns in curseforge_mod_image_cache
ALTER TABLE
  curseforge_mod_image_cache RENAME COLUMN metadataId TO metadata_id;

ALTER TABLE
  curseforge_mod_image_cache RENAME COLUMN url TO url;

ALTER TABLE
  curseforge_mod_image_cache RENAME COLUMN upToDate TO up_to_date;

-- Renaming columns in modrinth_mod_image_cache
ALTER TABLE
  modrinth_mod_image_cache RENAME COLUMN metadataId TO metadata_id;

ALTER TABLE
  modrinth_mod_image_cache RENAME COLUMN url TO url;

ALTER TABLE
  modrinth_mod_image_cache RENAME COLUMN upToDate TO up_to_date;

-- Renaming columns in curseforge_modpack_cache
ALTER TABLE
  curseforge_modpack_cache RENAME COLUMN projectId TO project_id;

ALTER TABLE
  curseforge_modpack_cache RENAME COLUMN fileId TO file_id;

ALTER TABLE
  curseforge_modpack_cache RENAME COLUMN modpackName TO modpack_name;

ALTER TABLE
  curseforge_modpack_cache RENAME COLUMN versionName TO version_name;

ALTER TABLE
  curseforge_modpack_cache RENAME COLUMN urlSlug TO url_slug;

ALTER TABLE
  curseforge_modpack_cache RENAME COLUMN updatedAt TO updated_at;

-- Renaming columns in modrinth_modpack_cache
ALTER TABLE
  modrinth_modpack_cache RENAME COLUMN projectId TO project_id;

ALTER TABLE
  modrinth_modpack_cache RENAME COLUMN versionId TO version_id;

ALTER TABLE
  modrinth_modpack_cache RENAME COLUMN modpackName TO modpack_name;

ALTER TABLE
  modrinth_modpack_cache RENAME COLUMN versionName TO version_name;

ALTER TABLE
  modrinth_modpack_cache RENAME COLUMN urlSlug TO url_slug;

ALTER TABLE
  modrinth_modpack_cache RENAME COLUMN updatedAt TO updated_at;

-- Renaming columns in curseforge_modpack_image_cache
ALTER TABLE
  curseforge_modpack_image_cache RENAME COLUMN projectId TO project_id;

ALTER TABLE
  curseforge_modpack_image_cache RENAME COLUMN fileId TO file_id;

ALTER TABLE
  curseforge_modpack_image_cache RENAME COLUMN url TO url;

-- Renaming columns in modrinth_modpack_image_cache
ALTER TABLE
  modrinth_modpack_image_cache RENAME COLUMN projectId TO project_id;

ALTER TABLE
  modrinth_modpack_image_cache RENAME COLUMN versionId TO version_id;

ALTER TABLE
  modrinth_modpack_image_cache RENAME COLUMN url TO url;