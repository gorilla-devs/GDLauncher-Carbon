import type { Page } from "playwright"
import { observeOperations } from "./operationObserver.js"
import { byInstanceName } from "./selectors.js"
export const RUNNING_REPAIR_REFUSAL =
  "Cannot repair while the instance is launching, queued, running, or being deleted"

/** The detail page has no library tile; inspect any mounted tile atomically. */
export function runningRepairGameCheck(
  page: Page,
  name: string,
  gamePid: number,
  isAlive: (pid: number) => boolean
): () => Promise<boolean> {
  return async () =>
    isAlive(gamePid) &&
    (await page.evaluate((selector) => {
      const tile = document.querySelector(selector)
      return (
        tile === null || tile.getAttribute("data-instance-state") === "running"
      )
    }, byInstanceName(name)))
}
/** Requires this request's serialized core refusal before reading invariants. */
export async function runRunningRepairCheck(
  page: Page,
  instance: number,
  isRunning: () => Promise<boolean>,
  confirm: () => Promise<void>,
  verify: () => Promise<void>,
  timeoutMs = 30_000
): Promise<void> {
  const observer = observeOperations(page)
  try {
    if (!(await isRunning()))
      throw new Error("Game stopped before repair confirmation")
    await confirm()
    const deadline = Date.now() + timeoutMs
    while (Date.now() < deadline) {
      if (!(await isRunning()))
        throw new Error("Game stopped before repair refusal")
      const response = await observer.find(
        "instance.repairModpack",
        (input) =>
          typeof input === "object" &&
          input !== null &&
          "instance" in input &&
          input.instance === instance
      )
      if (response) {
        const { result } = response
        const data = result.data as { message?: unknown } | undefined
        let error: { cause?: { display?: string }[] } | undefined
        try {
          if (typeof data?.message === "string")
            error = JSON.parse(data.message)
        } catch {
          /* diagnosed below */
        }
        if (
          result.type !== "error" ||
          !Array.isArray(error?.cause) ||
          !error.cause.some((cause) => cause.display === RUNNING_REPAIR_REFUSAL)
        ) {
          throw new Error(
            "Repair did not return the specific non-Inactive refusal"
          )
        }
        if (!(await isRunning()))
          throw new Error("Game stopped before repair refusal")
        await verify()
        return
      }
      await page.waitForTimeout(50)
    }
    throw new Error("Target repair refusal was not observed")
  } finally {
    observer.dispose()
  }
}
