import type {
  FEPaginatedShares,
  FEShareInfo,
  FEUpdateShareArgs
} from "@gd/core_module/bindings"
import { parseMaxDownloads } from "./validation"

/** The fields of a share that Edit Share can change. */
export type ShareChanges = Pick<FEUpdateShareArgs, "title" | "maxDownloads">

/**
 * What the form changes on a share. A field left as it was is left out, so
 * the server keeps it. An emptied title is sent as empty text and an emptied
 * limit as `null`: both remove the value.
 */
export function shareChanges(
  share: Pick<FEShareInfo, "title" | "maxDownloads">,
  form: { title: string; maxDownloads: string }
): ShareChanges {
  const changes: ShareChanges = {}
  const title = form.title.trim()
  if (title !== (share.title ?? "")) changes.title = title
  const maxDownloads = parseMaxDownloads(form.maxDownloads)
  if (maxDownloads !== (share.maxDownloads ?? null)) {
    changes.maxDownloads = maxDownloads
  }
  return changes
}

/** Whether the share as the server lists it now carries every change asked for. */
export function shareChangesApplied(
  listed: Pick<FEShareInfo, "title" | "maxDownloads">,
  changes: ShareChanges
): boolean {
  const titleApplied =
    changes.title === undefined || (listed.title ?? "") === changes.title
  const limitApplied =
    changes.maxDownloads === undefined ||
    (listed.maxDownloads ?? null) === changes.maxDownloads
  return titleApplied && limitApplied
}

/**
 * The share as a re-read of the list shows it. Undefined when the list could
 * not be read again or no longer holds the share: a failed re-read keeps the
 * rows it had, and those say nothing about what the server stored.
 */
export function listedShare(
  reread: { isError: boolean; data?: { pages: FEPaginatedShares[] } },
  shareCode: string
): FEShareInfo | undefined {
  if (reread.isError) return undefined
  return reread.data?.pages
    .flatMap((page) => page.items)
    .find((share) => share.shareCode === shareCode)
}
