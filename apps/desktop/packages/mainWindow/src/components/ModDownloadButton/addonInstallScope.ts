import type { TypedTFunction } from "@gd/i18n"

/** Toasts render addon titles as text, so i18next must not HTML-escape them
 *  ("Farmer&#39;s Delight"). */
const titleOptions = (title: string) => ({
  title,
  interpolation: { escapeValue: false }
})

/** Completion text for the selected installation destination. A data pack
 *  that lands in the instance (`staged`) says where to add it next, unless it
 *  was installed straight into `world`. */
export function addonInstallCompletion(
  t: TypedTFunction,
  title: string,
  staged = false,
  server = false,
  world?: string
): string {
  const options = titleOptions(title)
  if (!server && world)
    return t("content:_trn_worlds.added_to_world", {
      name: title,
      world,
      interpolation: { escapeValue: false }
    })
  if (!server && staged)
    return t("notifications:_trn_datapack_downloaded", options)
  return t("notifications:_trn_addon_installed", options)
}

export function addonInstallFailure(t: TypedTFunction, title: string): string {
  return t("notifications:_trn_addon_install_failed", titleOptions(title))
}

export function addonDownloadingToServer(
  t: TypedTFunction,
  title: string
): string {
  return t(
    "notifications:_trn_addon_downloading_to_server",
    titleOptions(title)
  )
}
