/**
 * Translation Key Helpers
 *
 * Type-safe helper functions to map enums and values to translation keys.
 * These replace all dynamic template literals with compile-time safe functions.
 */

import type {
  InstancesKeys,
  EnumsKeys,
  SettingsKeys,
  ErrorsKeys,
  TasksKeys,
  NamespacedTranslationKey
} from "./src/keys.generated"
import { supportedLanguages } from "./index"

/**
 * Supported language codes derived from supportedLanguages
 */
type SupportedLanguageCode = keyof typeof supportedLanguages

/**
 * Maps AddonType enum values to content:tabs.* translation keys
 * @param addonType - The addon type (mods, resourcepacks, shaders, datapacks, worlds)
 * @returns The fully qualified translation key
 */
export function getAddonTabKey(
  addonType: "mods" | "resourcepacks" | "shaders" | "datapacks" | "worlds"
):
  | "content:_trn_tabs.mods"
  | "content:_trn_tabs.resourcepacks"
  | "content:_trn_tabs.shaders"
  | "content:_trn_tabs.datapacks"
  | "content:_trn_tabs.worlds" {
  const keyMap = {
    mods: "content:_trn_tabs.mods",
    resourcepacks: "content:_trn_tabs.resourcepacks",
    shaders: "content:_trn_tabs.shaders",
    datapacks: "content:_trn_tabs.datapacks",
    worlds: "content:_trn_tabs.worlds"
  } as const

  return keyMap[addonType]
}

/**
 * Maps platform names to enums:* translation keys
 * @param platform - The platform name (curseforge or modrinth)
 * @returns The fully qualified translation key
 */
export function getPlatformKey(
  platform: "curseforge" | "modrinth"
): "enums:_trn_curseforge" | "enums:_trn_modrinth" {
  const keyMap = {
    curseforge: "enums:_trn_curseforge",
    modrinth: "enums:_trn_modrinth"
  } as const

  return keyMap[platform]
}

/**
 * Maps platform names to content:view_on_* translation keys
 * Used for context menu items when viewing mods on different platforms
 * @param platform - The platform name (curseforge or modrinth)
 * @returns The fully qualified translation key
 */
export function getViewOnKey(
  platform: "curseforge" | "modrinth"
): "content:_trn_view_on_curseforge" | "content:_trn_view_on_modrinth" {
  const keyMap = {
    curseforge: "content:_trn_view_on_curseforge",
    modrinth: "content:_trn_view_on_modrinth"
  } as const

  return keyMap[platform]
}

/**
 * Maps Xbox error codes to errors:xbox_* translation keys
 * @param errorCode - The Xbox error type (noAccount, xboxServicesBanned, adultVerificationRequired, childAccount)
 * @returns The fully qualified translation key
 */
export function getXboxErrorKey(
  errorCode:
    | "noAccount"
    | "xboxServicesBanned"
    | "adultVerificationRequired"
    | "childAccount"
):
  | "errors:_trn_xbox_noAccount"
  | "errors:_trn_xbox_xboxServicesBanned"
  | "errors:_trn_xbox_adultVerificationRequired"
  | "errors:_trn_xbox_childAccount" {
  const keyMap = {
    noAccount: "errors:_trn_xbox_noAccount",
    xboxServicesBanned: "errors:_trn_xbox_xboxServicesBanned",
    adultVerificationRequired: "errors:_trn_xbox_adultVerificationRequired",
    childAccount: "errors:_trn_xbox_childAccount"
  } as const

  return keyMap[errorCode]
}

/**
 * Maps enrollment error types to errors:* translation keys
 * @param errorType - The enrollment error type (deviceCodeExpired, xboxAccount, noGameOwnership, noGameProfile)
 * @returns The fully qualified translation key
 */
export function getEnrollmentErrorKey(
  errorType:
    | "deviceCodeExpired"
    | "xboxAccount"
    | "noGameOwnership"
    | "noGameProfile"
):
  | "errors:_trn_deviceCodeExpired"
  | "errors:_trn_xboxAccount"
  | "errors:_trn_noGameOwnership"
  | "errors:_trn_noGameProfile" {
  const keyMap = {
    deviceCodeExpired: "errors:_trn_deviceCodeExpired",
    xboxAccount: "errors:_trn_xboxAccount",
    noGameOwnership: "errors:_trn_noGameOwnership",
    noGameProfile: "errors:_trn_noGameProfile"
  } as const

  return keyMap[errorType]
}

/**
 * Maps theme IDs to settings:theme_* translation keys
 * @param themeId - The theme identifier (main, pixelato, win95, inferno, aether, frost)
 * @returns The fully qualified translation key
 */
export function getThemeKey(
  themeId: "main" | "pixelato" | "win95" | "inferno" | "aether" | "frost"
):
  | "settings:_trn_theme_main"
  | "settings:_trn_theme_pixelato"
  | "settings:_trn_theme_win95"
  | "settings:_trn_theme_inferno"
  | "settings:_trn_theme_aether"
  | "settings:_trn_theme_frost" {
  const keyMap: Record<string, any> = {
    main: "settings:_trn_theme_main",
    pixelato: "settings:_trn_theme_pixelato",
    win95: "settings:_trn_theme_win95",
    inferno: "settings:_trn_theme_inferno",
    aether: "settings:_trn_theme_aether",
    frost: "settings:_trn_theme_frost"
  } as const

  return keyMap[themeId] ?? ("settings:_trn_theme_main" as const)
}

/**
 * Maps language codes to languages:* translation keys
 * @param languageCode - The language code (from supportedLanguages)
 * @returns The fully qualified translation key
 */
export function getLanguageKey(
  languageCode: SupportedLanguageCode
): `languages:${SupportedLanguageCode}` {
  return `languages:${languageCode}` as const
}

/**
 * Maps language codes to languages:*_native translation keys
 * Used for displaying language names in their native language
 * @param languageCode - The language code (from supportedLanguages)
 * @returns The fully qualified translation key with _native suffix
 */
export function getLanguageNativeKey(
  languageCode: SupportedLanguageCode
): `languages:${SupportedLanguageCode}_native` {
  return `languages:${languageCode}_native` as const
}

/**
 * Maps Rust task/event types to task translation keys
 * Used by the frontend to display task status messages from Rust backend
 * @param taskType - The task type identifier from Translation enum
 * @returns The fully qualified translation key with _trn_ prefix
 */
export function getTaskTranslationKey(taskType: string): NamespacedTranslationKey {
  const mapping: Record<string, NamespacedTranslationKey> = {
    // Instance tasks - only include ones with actual translations
    'InstanceTaskLaunch': 'tasks:_trn_instance_task_launch',
    'InstanceTaskPrepare': 'tasks:_trn_instance_task_prepare',
    'InstanceTaskLaunchRequestVersions': 'tasks:_trn_instance_task_launch_request_versions',
    'InstanceTaskLaunchRequestModpack': 'tasks:_trn_instance_task_launch_request_modpack',
    'InstanceTaskLaunchDownloadModpack': 'tasks:_trn_instance_task_launch_download_modpack',
    'InstanceTaskLaunchDownloadModpackFiles': 'tasks:_trn_instance_task_launch_download_modpack_files',
    'InstanceTaskLaunchExtractModpackFiles': 'tasks:_trn_instance_task_launch_extract_modpack_files',
    'InstanceTaskLaunchRequestAddonMetadata': 'tasks:_trn_instance_task_launch_request_addon_metadata',
    'InstanceTaskLaunchApplyStagedPatches': 'tasks:_trn_instance_task_launch_apply_staged_patches',
    'InstanceTaskLaunchDownloadJava': 'tasks:_trn_instance_task_launch_download_java',
    'InstanceTaskLaunchExtractJava': 'tasks:_trn_instance_task_launch_extract_java',
    'InstanceTaskRequestModloaderInfo': 'tasks:_trn_instance_task_request_modloader_info',
    'InstanceTaskRequestMinecraftFiles': 'tasks:_trn_instance_task_request_minecraft_files',
    'InstanceTaskLaunchCheckingFiles': 'tasks:_trn_instance_task_launch_checking_files',
    'InstanceTaskLaunchDownloadFiles': 'tasks:_trn_instance_task_launch_download_files',
    'InstanceTaskGeneratingPackInfo': 'tasks:_trn_instance_task_generating_pack_info',
    'InstanceTaskFillCache': 'tasks:_trn_instance_task_fill_cache',
    'InstanceTaskLaunchExtractNatives': 'tasks:_trn_instance_task_launch_extract_natives',
    'InstanceTaskReconstructAssets': 'tasks:_trn_instance_task_reconstruct_assets',
    'InstanceTaskLaunchRunForgeProcessors': 'tasks:_trn_instance_task_launch_run_forge_processors',
    'InstanceTaskLaunchRunNeoforgeProcessors': 'tasks:_trn_instance_task_launch_run_neoforge_processors',
    'InstanceTaskInstallMod': 'tasks:_trn_instance_task_install_mod',
    'InstanceTaskLaunchInstallJava': 'tasks:_trn_instance_task_launch_install_java',
    'FinalizingImport': 'tasks:_trn_finalizing_import',
    'InstanceTaskLaunchWaiting': 'tasks:_trn_instance_task_launch_waiting',
  }
  // Default to a valid key if type isn't mapped
  return mapping[taskType] ?? ('tasks:_trn_instance_task_prepare' as NamespacedTranslationKey)
}
