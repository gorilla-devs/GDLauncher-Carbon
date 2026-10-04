import type { FETaskOutcome } from "@gd/core_module/bindings"
import { addonOutcomeText } from "./resolveTaskPoll"

/** Poll only the task ID returned by this accepted attempt. Read failures retry;
 * dismissal is deliberately left to the user. */
export async function waitAddonAttempt(
  taskId: number,
  read: (taskId: number) => Promise<FETaskOutcome | null>,
  current: () => boolean,
  wait: () => Promise<void> = () =>
    new Promise((resolve) => setTimeout(resolve, 500))
): Promise<
  | { kind: "completed" }
  | { kind: "failed"; message: string }
  | { kind: "abandoned" }
> {
  while (current()) {
    try {
      const outcome = await read(taskId)
      if (!current()) return { kind: "abandoned" }
      if (outcome?.type === "Succeeded") return { kind: "completed" }
      if (
        !outcome ||
        outcome.type === "Unavailable" ||
        outcome.type === "Dismissed"
      ) {
        return {
          kind: "failed",
          message:
            outcome?.type === "Dismissed"
              ? addonOutcomeText("notifications:_trn_addon_attempt_dismissed")
              : addonOutcomeText("notifications:_trn_addon_outcome_unavailable")
        }
      }
      if (
        outcome.type === "Failed" &&
        outcome.value.progress.type === "Failed"
      ) {
        return {
          kind: "failed",
          message:
            outcome.value.progress.value.cause[0]?.display ||
            addonOutcomeText("content:_trn_addon_update_failed")
        }
      }
    } catch {
      /* A failed read never certifies success. */
    }
    if (current()) await wait()
  }
  return { kind: "abandoned" }
}
