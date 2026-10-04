import { extractErrorCodeCause } from "../../../../utils/rspcErrorText"

/** The check before a version change was stopped, by the Cancel button or a
 *  quit. Nothing changed, so there is nothing to report. */
export function isChangeVersionCancelled(error: unknown): boolean {
  return extractErrorCodeCause(error)?.code === "MODPACK_CHECK_CANCELLED"
}

export type ChangeVersionRefusalText =
  | {
      key: "instances:_trn_change_version_unreachable"
      params: { platform: string }
    }
  | { key: "instances:_trn_change_version_unreachable_services" }
  | { key: "instances:_trn_change_version_gone" }

/** The translated wording of a refusal the core sends with a code (see
 *  `ChangeVersionRefusal` in imported_baseline.rs), or undefined for any other
 *  error, which is shown as the core words it. */
export function changeVersionRefusalText(
  error: unknown
): ChangeVersionRefusalText | undefined {
  const cause = extractErrorCodeCause(error)
  switch (cause?.code) {
    case "MODPACK_CHECK_UNREACHABLE": {
      const platform = (cause.data as { platform?: unknown } | null | undefined)
        ?.platform
      return typeof platform === "string"
        ? {
            key: "instances:_trn_change_version_unreachable",
            params: { platform }
          }
        : { key: "instances:_trn_change_version_unreachable_services" }
    }
    case "MODPACK_VERSION_GONE":
      return { key: "instances:_trn_change_version_gone" }
    default:
      return undefined
  }
}

/** The version to change to: what the player selected, as long as the list
 *  offers it. The installed version is preselected, but the platform may no
 *  longer serve it, and changing to a version it doesn't serve can only fail. */
export function offeredVersion(
  selected: string | null,
  versions: readonly { id: string }[]
): string | null {
  return selected !== null && versions.some((v) => v.id === selected)
    ? selected
    : null
}
