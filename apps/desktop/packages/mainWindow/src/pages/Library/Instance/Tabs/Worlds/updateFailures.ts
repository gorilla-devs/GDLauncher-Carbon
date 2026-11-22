import type { FEDatapackUpdateResult, FEWorld } from "@gd/core_module/bindings"
import { i18n } from "@gd/i18n"
import { toast } from "@gd/ui"
import { refusalText } from "../../../../../utils/refusalText"
import { labelOfFolder } from "../../../../../utils/worldLabel"

/** One toast naming every place "Update everywhere" did not update, with the
 *  first error beneath, worded from its code when it has one. Nothing when
 *  every copy was updated. */
export function toastUpdateFailures(
  results: readonly FEDatapackUpdateResult[],
  worlds?: readonly Pick<FEWorld, "name" | "title">[]
) {
  const failed = results.filter((result) => !result.ok)
  if (failed.length === 0) return
  const places = failed
    .map((result) =>
      result.place === null
        ? i18n.t("content:_trn_worlds.instance_copy")
        : labelOfFolder(worlds, result.place)
    )
    .join(", ")
  toast.error(
    i18n.t("content:_trn_worlds.update_failed_in", {
      places,
      interpolation: { escapeValue: false }
    }),
    {
      description: refusalText(
        failed[0].error_code,
        failed[0].error_data,
        failed[0].error
      )
    }
  )
}
