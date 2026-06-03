/**
 * i18n core: locale list, helpers, and the translation lookup `t()`.
 *
 * Translation source files live in src/lib/i18n/locales/<locale>.json.
 * Each file is a flat key→string map. Strings prefixed with `[NEEDS_REVIEW] `
 * have been machine-translated and need human review; the prefix is stripped
 * automatically before rendering and is invisible to users.
 */

import en from "./locales/en.json";
import ja from "./locales/ja.json";
import ko from "./locales/ko.json";
import de from "./locales/de.json";
import fr from "./locales/fr.json";
import es from "./locales/es.json";
import ptBR from "./locales/pt-BR.json";
import it from "./locales/it.json";
import { LOCALES, DEFAULT_LOCALE, LOCALIZED_PATH_PREFIXES, type Locale } from "./constants";

export { LOCALES, DEFAULT_LOCALE, LOCALIZED_PATH_PREFIXES };
export type { Locale };

const NEEDS_REVIEW_PREFIX = "[NEEDS_REVIEW] ";

const dictionaries: Record<Locale, Record<string, string>> = {
  en: en as Record<string, string>,
  ja: ja as Record<string, string>,
  ko: ko as Record<string, string>,
  de: de as Record<string, string>,
  fr: fr as Record<string, string>,
  es: es as Record<string, string>,
  "pt-BR": ptBR as Record<string, string>,
  it: it as Record<string, string>,
};

/**
 * Strip a URL pathname's locale prefix. `/ja/mods/foo` → `/mods/foo`.
 * Returns the path unchanged if it has no locale prefix (which is the
 * default-locale case. English lives at the root).
 */
export function pathWithoutLocale(pathname: string): string {
  const segments = pathname.split("/").filter(Boolean);
  if (segments.length === 0) return "/";
  const first = segments[0];
  if ((LOCALES as readonly string[]).includes(first) && first !== DEFAULT_LOCALE) {
    const rest = segments.slice(1).join("/");
    return rest ? `/${rest}` : "/";
  }
  return pathname;
}

/** Detect the locale from a URL pathname. Returns DEFAULT_LOCALE if none matches. */
export function getLocaleFromUrl(pathname: string): Locale {
  const segments = pathname.split("/").filter(Boolean);
  if (segments.length === 0) return DEFAULT_LOCALE;
  const first = segments[0];
  if ((LOCALES as readonly string[]).includes(first)) return first as Locale;
  return DEFAULT_LOCALE;
}

/**
 * Build a localized version of a path. The default locale lives at the root
 * (no prefix) so SEO equity stays on the canonical English URLs and the
 * existing inbound links keep working.
 */
export function localizedPath(path: string, locale: Locale): string {
  const clean = path.startsWith("/") ? path : `/${path}`;
  if (locale === DEFAULT_LOCALE) return clean === "/" ? "/" : clean;
  return clean === "/" ? `/${locale}` : `/${locale}${clean}`;
}

/** Translate a key for a given locale, falling back to English then to the key itself. */
export function t(locale: Locale, key: string): string {
  const dict = dictionaries[locale] ?? {};
  const fallback = dictionaries[DEFAULT_LOCALE] ?? {};
  const raw = dict[key] ?? fallback[key] ?? key;
  return raw.startsWith(NEEDS_REVIEW_PREFIX)
    ? raw.slice(NEEDS_REVIEW_PREFIX.length)
    : raw;
}

/** Build a translator bound to a specific locale (more ergonomic in templates). */
export function useTranslations(locale: Locale) {
  return (key: string) => t(locale, key);
}

/**
 * True if the given (locale-stripped) path has a localized version available.
 * The root `/` is always localized; everything else must match a known prefix.
 */
export function isPathLocalized(basePath: string): boolean {
  if (basePath === "/" || basePath === "") return true;
  return LOCALIZED_PATH_PREFIXES.some(
    (p) => basePath === p || basePath.startsWith(`${p}/`),
  );
}

/** Map our locale codes to the BCP-47 / OG locale format. */
export function getOgLocale(locale: Locale): string {
  switch (locale) {
    case "en":
      return "en_US";
    case "ja":
      return "ja_JP";
    case "ko":
      return "ko_KR";
    case "de":
      return "de_DE";
    case "fr":
      return "fr_FR";
    case "es":
      return "es_ES";
    case "pt-BR":
      return "pt_BR";
    case "it":
      return "it_IT";
  }
}

/** HTML lang attribute. BCP-47-compliant for every locale we ship. */
export function getHtmlLang(locale: Locale): string {
  return locale;
}

// Single regex that matches any absolute href whose first path segment is a
// localized prefix. Derived from LOCALIZED_PATH_PREFIXES so the two stay in
// lockstep. We deliberately don't match `/download/...` or `/api/...`, they
// don't have locale variants.
const LOCALIZED_HREF_REGEX = (() => {
  const alternation = LOCALIZED_PATH_PREFIXES
    .map((p) => p.slice(1).replace(/[.*+?^${}()|[\]\\]/g, "\\$&"))
    .join("|");
  return new RegExp(
    `\\bhref="(\\/(?:${alternation})(?:\\/[^"]*)?)"`,
    "g",
  );
})();

/**
 * Rewrite `href="/guides/foo"`-style links inside an HTML string so they
 * preserve the user's locale. Used by localized pages that render translated
 * prose via `set:html`, where the embedded hrefs would otherwise point at
 * the English (root) page and drop the locale.
 *
 * No-op for the default locale (English at root).
 */
export function localizeInternalLinks(html: string, locale: Locale): string {
  if (locale === DEFAULT_LOCALE) return html;
  return html.replace(LOCALIZED_HREF_REGEX, `href="/${locale}$1"`);
}

/** Human-readable locale name (used in the locale switcher). */
export const LOCALE_LABELS: Record<Locale, string> = {
  en: "English",
  ja: "日本語",
  ko: "한국어",
  de: "Deutsch",
  fr: "Français",
  es: "Español",
  "pt-BR": "Português (Brasil)",
  it: "Italiano",
};
