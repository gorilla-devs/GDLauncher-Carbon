import { closeAppAndCore, electronProcess } from "./electronApp"
const cleanExit = { exitCode: 0, signalCode: null }


it("a close whose Electron process exits non-zero fails strict cleanup after still confirming the core exit", async () => {
  const events: string[] = []
  const crashed = {
    evaluate: async () => 123,
    close: async () => {
      events.push("close")
    },
    // What an uncaught main-process exception leaves under
    // --gdl_disable_sentry: handleUncaughtException exits with 1.
    process: () => ({ exitCode: 1, signalCode: null })
  } as unknown as ElectronApplication
  await expect(
    closeAppAndCore(crashed, {
      strict: true,
      waitForExit: async (pid) => {
        events.push(`core-${pid}`)
      }
    })
  ).rejects.toThrow("Electron exited with code 1 instead of 0")
  expect(events).toEqual(["close", "core-123"])
  const logged: unknown[] = []
  const original = console.error
  console.error = (...args: unknown[]) => logged.push(args)
  try {
    await closeAppAndCore(crashed, { waitForExit: async () => {} })
  } finally {
    console.error = original
  }
  expect(logged.flat().map(String).join(" ")).toContain("exited with code 1")
})

it("reads the exit status from the process kept before close, since Playwright drops it on close", async () => {
  let closed = false
  // Playwright 1.58's process() looks up a dispatcher that close() disposes.
  const app = {
    evaluate: async () => 123,
    close: async () => {
      closed = true
    },
    process: () => {
      if (closed)
        throw new TypeError(
          "Cannot read properties of undefined (reading '_object')"
        )
      return cleanExit
    }
  } as unknown as ElectronApplication
  await closeAppAndCore(app, { strict: true, waitForExit: async () => {} })
  expect(electronProcess(app)).toBe(cleanExit)
})
