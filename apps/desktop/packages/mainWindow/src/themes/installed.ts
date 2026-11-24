import type { FEInstalledTheme } from "@gd/core_module/bindings"
import { themeErrorKey, type ThemeErrorKey } from "./errors"

/** What one installed theme's tile on the Appearance page shows. */
export interface InstalledTileModel {
  /** The settings key, "user:<id>". */
  key: string
  name: string
  author: string | null
  previewUrl: string | null
  /** rgb() of darkSlate-900, -700 and -600: the theme's, else main's. */
  shades: [string, string, string]
  /** False while the theme has an error. */
  selectable: boolean
  removable: boolean
  /** The saved theme. */
  active: boolean
  warning: { reason: ThemeErrorKey; detail: string | null } | null
}

/** A manifest text in the UI language, else the manifest's own text. */
export function localizedText(
  base: string,
  translations: Record<string, string>,
  language: string
): string {
  const own = Object.prototype.hasOwnProperty.call(translations, language)
    ? translations[language]
    : undefined
  return own || base
}

export function installedTileModel(
  t: FEInstalledTheme,
  savedKey: string | undefined,
  language: string,
  mainColors: Record<string, string>
): InstalledTileModel {
  const key = `user:${t.id}`
  const shade = (token: string) =>
    `rgb(${t.previewColors[token] ?? mainColors[token]})`
  const broken = t.error !== null || t.errorCode !== null
  return {
    key,
    name: localizedText(t.name ?? t.id, t.names, language),
    author: t.author,
    previewUrl: t.previewUrl,
    shades: [
      shade("darkSlate-900"),
      shade("darkSlate-700"),
      shade("darkSlate-600")
    ],
    selectable: !broken,
    removable: t.removable,
    active: savedKey === key,
    warning: broken
      ? { reason: themeErrorKey(t.errorCode), detail: t.error }
      : null
  }
}
