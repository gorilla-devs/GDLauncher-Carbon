import { FETask, Progress } from "@gd/core_module/bindings"

/** What the vtask poll effect should do for one `vtask.getTask` observation.
 *  Kept as plain data so the branching can be unit tested without a Solid
 *  reactive graph — the effect in `ModDownloadButton` interprets this to
 *  drive its signals/toasts/mutation. */
export type TaskPollAction =
  | { kind: "progress"; percent: number }
  | { kind: "failed"; message: string }
  | { kind: "completed"; showSuccessToast: boolean }
  | { kind: "noop" }

export interface TaskPollResult {
  action: TaskPollAction
  /** Next value for the `lastProgress` signal. */
  nextLastProgress: Progress | null
}

/** Decides what a poll of `vtask.getTask` means for the button.
 *
 *  `taskData` is `undefined` while the query hasn't resolved yet, `null`
 *  once the backend no longer knows the task (completed and forgotten), or
 *  the live `FETask` while it's tracked.
 *
 *  A task that fails never transitions to `null` on its own — it sits at
 *  `Failed` until dismissed. So `Failed` is handled here directly (own
 *  `"failed"` action) rather than waiting for a later `null` observation.
 *  `lastProgress` still gets threaded through so that if a `null` is ever
 *  observed for a task whose last known state was `Failed` (e.g. dismissed
 *  by another consumer of the same task before this effect saw the `null`
 *  itself), the success toast stays suppressed. */
export function resolveTaskPoll(
  taskData: FETask | null | undefined,
  lastProgress: Progress | null,
  isWorld: boolean
): TaskPollResult {
  const progress = taskData?.progress

  if (progress?.type === "Known") {
    return {
      action: { kind: "progress", percent: Math.round(progress.value * 100) },
      nextLastProgress: progress
    }
  }

  if (progress?.type === "Failed") {
    const message = progress.value.cause[0]?.display
    return {
      action: { kind: "failed", message: message || "" },
      nextLastProgress: progress
    }
  }

  if (taskData === null) {
    return {
      action: {
        kind: "completed",
        showSuccessToast: isWorld && lastProgress?.type !== "Failed"
      },
      nextLastProgress: null
    }
  }

  return { action: { kind: "noop" }, nextLastProgress: lastProgress }
}
