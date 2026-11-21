import { FETaskOutcome, Progress } from "@gd/core_module/bindings"
import { i18n, type NamespacedTranslationKey } from "@gd/i18n"
import { refusalText } from "../../../utils/refusalText"

/** Text for an outcome the launcher reports itself. Callers count a failure
 *  by its message, so it is never empty: without a loaded bundle the key
 *  itself stands in. */
export function addonOutcomeText(key: NamespacedTranslationKey): string {
  return i18n.t(key) || key
}

export type TaskPollAction =
  | { kind: "progress"; percent: number }
  | {
      kind: "failed"
      message: string
      /** The code of the core's refusal, when the task failed with one. */
      code?: string
    }
  | { kind: "completed"; showSuccessToast: boolean }
  | { kind: "dismissed" }
  | { kind: "noop" }

export interface TaskPollResult {
  action: TaskPollAction
  nextLastProgress: Progress | null
}

/** Only explicit success certifies an accepted attempt. Missing history and
 * acknowledged failure cannot become success for a late observer. */
export function resolveTaskPoll(
  outcome: FETaskOutcome | null | undefined,
  lastProgress: Progress | null,
  showSuccessToast: boolean
): TaskPollResult {
  if (outcome === undefined)
    return { action: { kind: "noop" }, nextLastProgress: lastProgress }
  if (outcome?.type === "Succeeded")
    return {
      action: { kind: "completed", showSuccessToast },
      nextLastProgress: null
    }
  if (outcome?.type === "Dismissed")
    return { action: { kind: "dismissed" }, nextLastProgress: null }
  if (!outcome || outcome.type === "Unavailable")
    return {
      action: {
        kind: "failed",
        message: addonOutcomeText(
          "notifications:_trn_addon_outcome_unavailable"
        )
      },
      nextLastProgress: null
    }
  const progress = outcome.value.progress
  if (progress.type === "Failed") {
    // A failed task keeps the code and data of the refusal it failed with.
    const cause = progress.value.cause[0]
    return {
      action: {
        kind: "failed",
        message: refusalText(cause?.code, cause?.data, cause?.display) || "",
        ...(cause?.code && { code: cause.code })
      },
      nextLastProgress: progress
    }
  }
  if (progress.type === "Known")
    return {
      action: { kind: "progress", percent: Math.round(progress.value * 100) },
      nextLastProgress: progress
    }
  return { action: { kind: "noop" }, nextLastProgress: progress }
}
