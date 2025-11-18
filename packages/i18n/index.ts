import {
  Trans as OriginalTrans,
  TransProps as OriginalTransProps,
  useTransContext as originalUseTransContext,
  TransProvider
} from "@mbarzda/solid-i18next"
import { ParentComponent } from "solid-js"
import type { TOptions } from "i18next"
import type { NamespacedTranslationKey } from "./src/keys.generated"
import languagesData from "./languages.json" with { type: "json" }

export const supportedLanguages = {
  english: "united-states",
  italian: "italy"
}

type NamespacesMap = {
  [key: string]: Object
}

const loadLanguageFiles = async (lang: string) => {
  const namespacesMap: NamespacesMap = {}

  // Load languages data (same for all languages, not language-specific)
  namespacesMap["languages"] = languagesData

  // All translation namespaces (alphabetically sorted for consistency)
  const namespaces = [
    "accounts",
    "ads",
    "app",
    "auth",
    "content",
    "enums",
    "errors",
    "general",
    "instances",
    "java",
    "library",
    "logs",
    "modals",
    "news",
    "notifications",
    "onboarding",
    "placeholders",
    "search",
    "settings",
    "tasks",
    "tracking",
    "ui",
    "window"
  ]

  for (const namespace of namespaces) {
    try {
      const language = (await import(`./locale/${lang}/${namespace}.json`))
        .default

      namespacesMap[namespace] = language
    } catch (err) {
      // Silently skip missing language files - i18next fallbackLng will handle it
    }
  }

  return namespacesMap
}

// ============================================================================
// TYPE-SAFE TRANSLATION COMPONENTS
// ============================================================================

/**
 * Type-safe translation component with autocomplete support.
 *
 * Use namespace:key format (e.g., "auth:login.welcome_to")
 * or just key for default namespace.
 *
 * All dynamic translation keys are now handled via helper functions
 * (e.g., getAddonTabKey(), getPlatformKey(), getThemeKey()).
 *
 * @example
 * <Trans key="auth:login.welcome_to" />
 * <Trans key="general:continue" />
 * <Trans key={getThemeKey(themeId)} />
 * <Trans key={getAddonTabKey(addonType)} />
 */
export type TypedTransProps = Omit<OriginalTransProps, "key"> & {
  key: NamespacedTranslationKey | HelperFunctionKeys
}

export const Trans: ParentComponent<TypedTransProps> = OriginalTrans as any

/**
 * Translation keys returned by helper functions.
 * These are type-safe mappings from enums to actual translation keys.
 */
type HelperFunctionKeys =
  | `content:tabs.${'mods' | 'resourcepacks' | 'shaders' | 'datapacks' | 'worlds'}`
  | `platforms:${'curseforge' | 'modrinth'}`
  | `content:view_on_${'curseforge' | 'modrinth'}`
  | `errors:xbox_${'noAccount' | 'xboxServicesBanned' | 'adultVerificationRequired' | 'childAccount'}`
  | `errors:${'deviceCodeExpired' | 'xboxAccount' | 'noGameOwnership' | 'noGameProfile'}`
  | `settings:theme_${'main' | 'pixelato' | 'win95' | 'inferno' | 'aether' | 'frost'}`
  | `languages:${'english' | 'french' | 'german' | 'japanese' | 'italian' | 'spanish' | 'portuguese' | 'russian' | 'arabic' | 'chinese' | 'hindi' | 'turkish' | 'thai' | 'vietnamese' | 'korean' | 'polish' | 'dutch' | 'hungarian' | 'swedish' | 'czech' | 'catalan' | 'greek' | 'danish' | 'finnish' | 'norwegian' | 'bulgarian' | 'serbian' | 'ukrainian' | 'hebrew' | 'romanian' | 'indonesian' | 'croatian' | 'latvian' | 'slovak' | 'slovenian' | 'lithuanian' | 'filipino' | 'bengali' | 'malay' | 'persian' | 'afrikaans' | 'albanian' | 'armenian' | 'azerbaijani' | 'basque' | 'belarusian'}`
  | `languages:${'english' | 'french' | 'german' | 'japanese' | 'italian' | 'spanish' | 'portuguese' | 'russian' | 'arabic' | 'chinese' | 'hindi' | 'turkish' | 'thai' | 'vietnamese' | 'korean' | 'polish' | 'dutch' | 'hungarian' | 'swedish' | 'czech' | 'catalan' | 'greek' | 'danish' | 'finnish' | 'norwegian' | 'bulgarian' | 'serbian' | 'ukrainian' | 'hebrew' | 'romanian' | 'indonesian' | 'croatian' | 'latvian' | 'slovak' | 'slovenian' | 'lithuanian' | 'filipino' | 'bengali' | 'malay' | 'persian' | 'afrikaans' | 'albanian' | 'armenian' | 'azerbaijani' | 'basque' | 'belarusian'}_native`

/**
 * Type-safe translation function with autocomplete support.
 *
 * Returns a typed t() function that accepts valid translation keys.
 * All dynamic translation keys are now handled via helper functions.
 *
 * @example
 * const [t] = useTransContext()
 * t("auth:login.welcome_to")
 * t("general:continue")
 * t("instance:action_play", { count: 5 })
 * t(getThemeKey(themeId))           // ✅ Type-safe helper function
 * t(getAddonTabKey(addonType))      // ✅ Type-safe helper function
 */
export type TypedTFunction = {
  (key: NamespacedTranslationKey | HelperFunctionKeys, options?: TOptions): string
  (key: NamespacedTranslationKey | HelperFunctionKeys, defaultValue?: string, options?: TOptions): string
}

export const useTransContext = (): [
  TypedTFunction,
  ReturnType<typeof originalUseTransContext>[1]
] => {
  return originalUseTransContext() as any
}

// ============================================================================
// EXPORTS
// ============================================================================

export { TransProvider }
export { default as i18n } from "i18next"
export { loadLanguageFiles }

// Export generated types for advanced use cases
export type {
  NamespacedTranslationKey,
  TranslationKey,
  AccountsKeys,
  AdsKeys,
  AppKeys,
  AuthKeys,
  ContentKeys,
  EnumsKeys,
  ErrorsKeys,
  GeneralKeys,
  InstancesKeys,
  JavaKeys,
  LibraryKeys,
  LogsKeys,
  ModalsKeys,
  NewsKeys,
  NotificationsKeys,
  OnboardingKeys,
  PlaceholdersKeys,
  SearchKeys,
  SettingsKeys,
  TasksKeys,
  TrackingKeys,
  UiKeys,
  WindowKeys
} from "./src/keys.generated"
