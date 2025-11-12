/**
 * Translation Key Helpers
 *
 * Type-safe helper functions to map enums and values to translation keys.
 * These replace all dynamic template literals with compile-time safe functions.
 */

import type {
  InstanceKeys,
  PlatformsKeys,
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
 * Maps AddonType enum values to instance:tabs.* translation keys
 * @param addonType - The addon type (mods, resourcepacks, shaders, datapacks, worlds)
 * @returns The fully qualified translation key
 */
export function getAddonTabKey(
  addonType: "mods" | "resourcepacks" | "shaders" | "datapacks" | "worlds"
):
  | "instance:_trn_tabs.mods"
  | "instance:_trn_tabs.resourcepacks"
  | "instance:_trn_tabs.shaders"
  | "instance:_trn_tabs.datapacks"
  | "instance:_trn_tabs.worlds" {
  const keyMap = {
    mods: "instance:_trn_tabs.mods",
    resourcepacks: "instance:_trn_tabs.resourcepacks",
    shaders: "instance:_trn_tabs.shaders",
    datapacks: "instance:_trn_tabs.datapacks",
    worlds: "instance:_trn_tabs.worlds"
  } as const

  return keyMap[addonType]
}

/**
 * Maps platform names to platforms:* translation keys
 * @param platform - The platform name (curseforge or modrinth)
 * @returns The fully qualified translation key
 */
export function getPlatformKey(
  platform: "curseforge" | "modrinth"
): "platforms:_trn_curseforge" | "platforms:_trn_modrinth" {
  const keyMap = {
    curseforge: "platforms:_trn_curseforge",
    modrinth: "platforms:_trn_modrinth"
  } as const

  return keyMap[platform]
}

/**
 * Maps platform names to instance:view_on_* translation keys
 * Used for context menu items when viewing mods on different platforms
 * @param platform - The platform name (curseforge or modrinth)
 * @returns The fully qualified translation key
 */
export function getViewOnKey(
  platform: "curseforge" | "modrinth"
): "instance:_trn_view_on_curseforge" | "instance:_trn_view_on_modrinth" {
  const keyMap = {
    curseforge: "instance:_trn_view_on_curseforge",
    modrinth: "instance:_trn_view_on_modrinth"
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
    'InstanceTaskLaunch': 'tasks:_trn_InstanceTaskLaunch',
    'InstanceTaskPrepare': 'tasks:_trn_InstanceTaskPrepare',
    'InstanceTaskLaunchRequestVersions': 'tasks:_trn_InstanceTaskLaunchRequestVersions',
    'InstanceTaskLaunchRequestModpack': 'tasks:_trn_InstanceTaskLaunchRequestModpack',
    'InstanceTaskLaunchDownloadModpack': 'tasks:_trn_InstanceTaskLaunchDownloadModpack',
    'InstanceTaskLaunchDownloadModpackFiles': 'tasks:_trn_InstanceTaskLaunchDownloadModpackFiles',
    'InstanceTaskLaunchExtractModpackFiles': 'tasks:_trn_InstanceTaskLaunchExtractModpackFiles',
    'InstanceTaskLaunchRequestAddonMetadata': 'tasks:_trn_InstanceTaskLaunchRequestAddonMetadata',
    'InstanceTaskLaunchApplyStagedPatches': 'tasks:_trn_InstanceTaskLaunchApplyStagedPatches',
    'InstanceTaskLaunchDownloadJava': 'tasks:_trn_InstanceTaskLaunchDownloadJava',
    'InstanceTaskLaunchExtractJava': 'tasks:_trn_InstanceTaskLaunchExtractJava',
    'InstanceTaskRequestModloaderInfo': 'tasks:_trn_InstanceTaskRequestModloaderInfo',
    'InstanceTaskRequestMinecraftFiles': 'tasks:_trn_InstanceTaskRequestMinecraftFiles',
    'InstanceTaskLaunchCheckingFiles': 'tasks:_trn_InstanceTaskLaunchCheckingFiles',
    'InstanceTaskLaunchDownloadFiles': 'tasks:_trn_InstanceTaskLaunchDownloadFiles',
    'InstanceTaskGeneratingPackInfo': 'tasks:_trn_InstanceTaskGeneratingPackInfo',
    'InstanceTaskFillCache': 'tasks:_trn_InstanceTaskFillCache',
    'InstanceTaskLaunchExtractNatives': 'tasks:_trn_InstanceTaskLaunchExtractNatives',
    'InstanceTaskReconstructAssets': 'tasks:_trn_InstanceTaskReconstructAssets',
    'InstanceTaskLaunchRunForgeProcessors': 'tasks:_trn_InstanceTaskLaunchRunForgeProcessors',
    'InstanceTaskLaunchRunNeoforgeProcessors': 'tasks:_trn_InstanceTaskLaunchRunNeoforgeProcessors',
    'InstanceTaskInstallMod': 'tasks:_trn_InstanceTaskInstallMod',
    'InstanceTaskLaunchInstallJava': 'tasks:_trn_InstanceTaskLaunchInstallJava',
    'FinalizingImport': 'tasks:_trn_FinalizingImport',
    'InstanceTaskLaunchWaiting': 'tasks:_trn_InstanceTaskLaunchWaiting',
  }
  // Default to a valid key if type isn't mapped
  return mapping[taskType] ?? ('tasks:_trn_InstanceTaskPrepare' as NamespacedTranslationKey)
}
