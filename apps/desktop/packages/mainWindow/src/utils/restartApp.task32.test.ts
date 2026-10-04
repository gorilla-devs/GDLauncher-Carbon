import { expect, it, vi } from "vitest"
import { createRoot } from "solid-js"
import { createRestart } from "./restartApp"

function deferred() {
  let resolve!: (value: unknown) => void
  let reject!: (error: unknown) => void
  const promise = new Promise((res, rej) => {
    resolve = res
    reject = rej
  })
  return { promise, resolve, reject }
}

it("a second click while a restart still runs neither restarts again nor reports a failure", async () => {
  const pending = deferred()
  const relaunch = vi.fn(() => pending.promise)
  const failed = vi.fn()
  await createRoot(async (dispose) => {
    const { restarting, restart } = createRestart(relaunch, failed)
    expect(restarting()).toBe(false)
    const first = restart()
    expect(restarting()).toBe(true)
    await restart()
    expect(relaunch).toHaveBeenCalledOnce()
    expect(failed).not.toHaveBeenCalled()
    // Keep GDLauncher open: the button works again.
    pending.resolve({ status: "kept" })
    await first
    expect(restarting()).toBe(false)
    expect(failed).not.toHaveBeenCalled()
    dispose()
  })
})

it("a restart that failed outright is reported once and can be tried again", async () => {
  const pending = deferred()
  const relaunch = vi.fn(() => pending.promise)
  const failed = vi.fn()
  await createRoot(async (dispose) => {
    const { restarting, restart } = createRestart(relaunch, failed)
    const first = restart()
    const error = new Error("core exit was not confirmed after SIGKILL")
    pending.reject(error)
    await first
    expect(failed).toHaveBeenCalledExactlyOnceWith(error)
    expect(restarting()).toBe(false)
    relaunch.mockResolvedValueOnce({ status: "relaunching" })
    await restart()
    expect(relaunch).toHaveBeenCalledTimes(2)
    dispose()
  })
})
