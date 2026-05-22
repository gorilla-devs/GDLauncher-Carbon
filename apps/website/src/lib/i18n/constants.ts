/**
 * Locale list and path-prefix constants shared between the runtime i18n
 * module and astro.config.ts. Kept dependency-free (no JSON imports) so
 * the config loader doesn't have to parse the locale dictionaries just
 * to read these values.
 */

export const LOCALES = ["en", "ja", "ko", "de", "fr", "es", "pt-BR", "it"] as const;
export type Locale = (typeof LOCALES)[number];
export const DEFAULT_LOCALE: Locale = "en";

/**
 * Path prefixes that have a `[locale]/` counterpart. Used by the locale
 * switcher, the hreflang emitter, and astro.config.ts (sitemap building).
 *
 * Note: the legal/confirmation/share pages keep their body in English (legal
 * accuracy + short utility copy), but exist under `[locale]/...` so the
 * Header/Footer/locale-switcher render in the user's locale instead of
 * silently dropping them back to English.
 *
 * Keep this list in sync with the directory tree under `src/pages/[locale]/`.
 * Order matters: longer prefixes must come before shorter ones so that
 * `/docs/...` matches before bare `/`.
 *
 * Addon detail pages (`/[type]/[platform]/[slug]`) are covered implicitly
 * because every valid `[type]` (mods, modpacks, datapacks, resourcepacks,
 * shaders, worlds) appears here as a top-level prefix.
 */
export const LOCALIZED_PATH_PREFIXES = [
  "/datapacks",
  "/docs",
  "/guides",
  "/modpacks",
  "/mods",
  "/resourcepacks",
  "/shaders",
  "/worlds",
  "/blog",
  "/vs",
  "/best-minecraft-launcher",
  "/fabric-launcher",
  "/forge-launcher",
  "/minecraft-mod-launcher",
  "/minecraft-modpack-launcher",
  "/terms-of-service",
  "/privacy-statement",
  "/user-account-confirmed",
  "/user-deletion-confirmed",
  "/newsletter/confirm",
  "/instance-share",
] as const;
