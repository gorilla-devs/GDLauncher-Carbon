import type { FETaskOutcome } from "@gd/core_module/bindings"
import { extractErrorDisplay } from "../../../utils/rspcErrorText"
import { waitAddonAttempt } from "./waitAddonAttempt"

/** How many updates of a batch run at once. The core takes several updates of
 * one instance at a time (as 2.0.40's overlapping downloads did); a few keep a
 * long batch quick without sending every lookup and download at once. */
export const ADDON_UPDATE_CONCURRENCY = 4

export async function updateAddonBatch<T>(
  items: readonly T[],
  accept: (item: T) => Promise<number>,
  read: (taskId: number) => Promise<FETaskOutcome | null>,
  current: () => boolean,
  settled: (item: T, error?: string) => void,
  wait?: () => Promise<void>,
  options: {
    concurrency?: number
    /** Called as an update is sent, before its outcome is known. Updates that
     * never get here never ran. */
    started?: (item: T) => void
  } = {}
) {
  let failed = 0
  let completed = 0
  let abandoned = false
  let next = 0
  // Each worker sends one update and waits for its own outcome before taking
  // the next, so at most `concurrency` are in flight and every outcome is
  // still reported by name.
  const worker = async () => {
    while (next < items.length) {
      if (!current()) {
        abandoned = true
        return
      }
      const item = items[next++]
      options.started?.(item)
      let error: string | undefined
      try {
        const taskId = await accept(item)
        const outcome = await waitAddonAttempt(taskId, read, current, wait)
        if (outcome.kind === "abandoned") {
          abandoned = true
          return
        }
        if (outcome.kind === "failed") error = outcome.message
      } catch (cause) {
        // The core's sentence, not the rspc payload with its backtrace.
        error = extractErrorDisplay(cause)
      }
      if (!current()) {
        abandoned = true
        return
      }
      if (error !== undefined) failed++
      completed++
      settled(item, error)
    }
  }
  const workers = Math.max(
    1,
    Math.min(options.concurrency ?? ADDON_UPDATE_CONCURRENCY, items.length)
  )
  await Promise.all(Array.from({ length: workers }, worker))
  return { abandoned, failed, completed }
}
