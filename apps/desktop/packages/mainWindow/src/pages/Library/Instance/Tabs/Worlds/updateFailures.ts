import type { FEDatapackUpdateResult } from "@gd/core_module/bindings"
import { i18n } from "@gd/i18n"
import { toast } from "@gd/ui"

/** One toast naming every place "Update everywhere" did not update, with the
 *  first error beneath. Nothing when every copy was updated. */
export function toastUpdateFailures(
  results: readonly FEDatapackUpdateResult[]
) {
  const failed = results.filter((result) => !result.ok)
  if (failed.length === 0) return
  const places = failed
    .map(
      (result) => result.place ?? i18n.t("content:_trn_worlds.instance_copy")
    )
    .join(", ")
  toast.error(
    i18n.t("content:_trn_worlds.update_failed_in", {
      places,
      interpolation: { escapeValue: false }
    }),
    { description: failed[0].error ?? undefined }
  )
}
