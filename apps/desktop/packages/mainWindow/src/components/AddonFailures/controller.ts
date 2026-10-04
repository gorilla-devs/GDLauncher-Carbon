import type { FETask, FETaskOutcome } from "@gd/core_module/bindings"

// The manager retains details across navigation. No UI close action dismisses.
export function retainedAddonFailures(tasks: readonly FETask[]): FETask[] {
  return tasks.filter(
    (task) =>
      task.progress.type === "Failed" &&
      [
        "InstanceTaskInstallMod",
        "InstanceTaskStageDatapack",
        "InstanceTaskInstallWorldCopy",
        "ServerTaskInstallMod"
      ].includes(task.name.translation)
  )
}

export async function acknowledgeCapturedFailure(
  taskId: number,
  read: (id: number) => Promise<FETaskOutcome>,
  dismiss: (id: number) => Promise<unknown>
): Promise<void> {
  const outcome = await read(taskId)
  if (outcome.type === "Dismissed") return
  if (outcome.type !== "Failed")
    throw new Error("This attempt is not a retained failure")
  await dismiss(taskId)
}

/** Removes each listed failure in turn. One that cannot be removed does not
 * keep the others in the list; resolves to how many could not be removed. */
export async function acknowledgeCapturedFailures(
  taskIds: readonly number[],
  read: (id: number) => Promise<FETaskOutcome>,
  dismiss: (id: number) => Promise<unknown>
): Promise<number> {
  let failed = 0
  for (const id of taskIds) {
    try {
      await acknowledgeCapturedFailure(id, read, dismiss)
    } catch (error) {
      console.error("Failed to dismiss addon failure:", error)
      failed++
    }
  }
  return failed
}

/** Retry is explicit acknowledgement of the captured failed attempt, before
 * another attempt is accepted. Read/dismiss failures do not start a retry. */
export async function retryCapturedAddonFailure(
  taskId: number | null,
  read: (id: number) => Promise<FETaskOutcome>,
  dismiss: (id: number) => Promise<unknown>,
  retry: () => Promise<void>
): Promise<void> {
  if (taskId !== null) await acknowledgeCapturedFailure(taskId, read, dismiss)
  await retry()
}
