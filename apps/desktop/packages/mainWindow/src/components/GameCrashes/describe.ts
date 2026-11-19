import type { FEGameCrash } from "@gd/core_module/bindings"
import type { NamespacedTranslationKey } from "@gd/i18n"

export type Translate = (
  key: NamespacedTranslationKey,
  options?: Record<string, unknown>
) => string

// Instance names are the user's own, and the toast renders text.
const noEscape = { escapeValue: false }

/** What the notice says of a game that crashed or did not start: which
 * instance, and the one thing known about how it ended. */
export function describeGameCrash(
  crash: FEGameCrash,
  t: Translate
): { title: string; description?: string } {
  const instance = crash.name
  if (crash.kind.type === "Crashed") {
    return {
      title: t("notifications:_trn_game_crashed", {
        instance,
        interpolation: noEscape
      }),
      ...(crash.kind.value.report
        ? { description: t("notifications:_trn_game_crashed_report") }
        : {})
    }
  }
  const { seconds, code } = crash.kind.value
  return {
    title: t("notifications:_trn_game_did_not_start", {
      instance,
      interpolation: noEscape
    }),
    description:
      code === null
        ? t("notifications:_trn_game_closed_early", { count: seconds })
        : t("notifications:_trn_game_closed_early_code", {
            count: seconds,
            code
          })
  }
}
