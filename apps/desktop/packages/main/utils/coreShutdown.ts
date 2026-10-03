import type { ChildProcess } from "node:child_process"
import { randomUUID } from "node:crypto"
import { canonicalRuntimePath } from "./runtimeRelocation"

/** Timeout/error/signal failure are failure, never process-exit proof. */
export function terminateAndObserveCoreExit(
  proc: ChildProcess,
  timeoutMs = 5000
): Promise<void> {
  if (proc.exitCode !== null || proc.signalCode !== null)
    return Promise.resolve()
  return new Promise((resolve, reject) => {
    let settled = false
    const finish = (error?: Error) => {
      if (settled) return
      settled = true
      clearTimeout(timer)
      proc.off("exit", exited)
      proc.off("error", failed)
      if (error) reject(error)
      else resolve()
    }
    const exited = () => finish()
    const failed = (error: Error) => finish(error)
    const timer = setTimeout(
      () =>
        finish(
          new Error("Core exit was not confirmed; runtime source retained")
        ),
      timeoutMs
    )
    proc.once("exit", exited)
    proc.once("error", failed)
    if (proc.exitCode !== null || proc.signalCode !== null) {
      finish()
      return
    }
    try {
      if (!proc.kill()) finish(new Error("Failed to signal core shutdown"))
    } catch (error) {
      finish(error as Error)
    }
  })
}
export interface RelocationCore {
  port: number
  apiToken: string
  process: ChildProcess | null
}
export async function stopCoreForRelocation(
  core: RelocationCore,
  source: string,
  transport: typeof fetch = fetch,
  timeoutMs = 5000
): Promise<void> {
  const proc = core.process
  if (!proc || proc.exitCode !== null || proc.signalCode !== null) {
    throw new Error(
      "Relocation requires a live launcher-owned core; restart normally and retry"
    )
  }
  const requestId = randomUUID()
  async function request(endpoint: string) {
    const response = await transport(
      `http://127.0.0.1:${core.port}/runtime/${endpoint}`,
      {
        method: "POST",
        headers: {
          "content-type": "application/json",
          "x-api-token": core.apiToken
        },
        body: JSON.stringify({ requestId }),
        signal: AbortSignal.timeout(timeoutMs)
      }
    )
    if (!response.ok)
      throw new Error(`Core relocation refused: ${await response.text()}`)
    const data = await response.json()
    if (data.requestId !== requestId)
      throw new Error("Core relocation acknowledgment mismatch")
    return data
  }
  let prepared = false,
    shutdown = false
  try {
    const preparation = await request("prepare-relocation")
    prepared = true
    if (
      (await canonicalRuntimePath(preparation.runtimePath)) !==
      (await canonicalRuntimePath(source))
    ) {
      throw new Error("Core runtime differs from the selected source")
    }
    await request("shutdown")
    shutdown = true
    await terminateAndObserveCoreExit(proc, timeoutMs)
  } catch (error) {
    if (prepared && !shutdown)
      await request("cancel-relocation").catch(() => {})
    throw error
  }
}
