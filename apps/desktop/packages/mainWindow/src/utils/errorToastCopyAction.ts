import { toast } from "@gd/ui"
import { i18n } from "@gd/i18n"

/** The Copy action on an error toast: copies `text`, the error a user pastes
 *  into a bug report, and confirms. Shared by the global error toast
 *  (rspcClient.ts) and a mutation's own failure toast (mutationErrorToast.ts),
 *  and kept apart from both so neither has to load the other. The `||`
 *  fallbacks cover errors fired before i18n has initialized, where t()
 *  returns undefined. */
export function errorToastCopyAction(text: string) {
  return {
    label: i18n.t("general:_trn_copy") || "Copy",
    onClick: () => {
      navigator.clipboard
        .writeText(text)
        .then(() =>
          toast.success(
            i18n.t("general:_trn_general_copied_to_clipboard") ||
              "Copied to clipboard"
          )
        )
        .catch(() => {})
    }
  }
}
