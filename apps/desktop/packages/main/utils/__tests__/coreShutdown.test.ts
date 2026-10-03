import { it, expect, vi, afterEach } from "vitest"
import { EventEmitter } from "node:events"
import {
  terminateAndObserveCoreExit,
  stopCoreForRelocation
} from "../coreShutdown"
import type { ChildProcess } from "node:child_process"
afterEach(() => vi.useRealTimers())
function processFixture() {
  const child = Object.assign(new EventEmitter(), {
    exitCode: null,
    signalCode: null,
    kill: vi.fn(() => true)
  })
  return { child, proc: child as unknown as ChildProcess }
}
it("observes exit after listeners were attached before signaling", async () => {
  const { child, proc } = processFixture()
  child.kill.mockImplementation(() => {
    child.emit("exit", 0)
    return true
  })
  await terminateAndObserveCoreExit(proc)
  expect(child.listenerCount("exit")).toBe(0)
  expect(child.listenerCount("error")).toBe(0)
})
it("timeout is rejection without pretending process exited", async () => {
  vi.useFakeTimers()
  const { child, proc } = processFixture()
  const pending = expect(terminateAndObserveCoreExit(proc, 10)).rejects.toThrow(
    /not confirmed/
  )
  await vi.advanceTimersByTimeAsync(10)
  await pending
  expect(child.listenerCount("exit")).toBe(0)
})
it.each(["false", "throw", "error"])("rejects signal %s", async (kind) => {
  const { child, proc } = processFixture()
  child.kill.mockImplementation(() => {
    if (kind === "throw") throw new Error("signal error")
    if (kind === "error") child.emit("error", new Error("process error"))
    return false
  })
  await expect(terminateAndObserveCoreExit(proc)).rejects.toThrow()
  expect(child.listenerCount("error")).toBe(0)
})
it("uses authenticated matching handshake before termination", async () => {
  const { child, proc } = processFixture()
  const calls: string[] = []
  child.kill.mockImplementation(() => {
    calls.push("kill")
    child.emit("exit", 0)
    return true
  })
  const transport = vi.fn(async (url, options) => {
    const endpoint = String(url).split("/").at(-1)!
    calls.push(endpoint)
    expect(options.headers["x-api-token"]).toBe("synthetic-token")
    return {
      ok: true,
      json: async () => ({
        ...JSON.parse(options.body),
        runtimePath: process.cwd()
      })
    }
  })
  await stopCoreForRelocation(
    { port: 1, apiToken: "synthetic-token", process: proc },
    process.cwd(),
    transport as any
  )
  expect(calls).toEqual(["prepare-relocation", "shutdown", "kill"])
})
it("busy core does not receive kill", async () => {
  const { child, proc } = processFixture()
  await expect(
    stopCoreForRelocation(
      { port: 1, apiToken: "fixture", process: proc },
      process.cwd(),
      vi.fn(async () => ({
        ok: false,
        text: async () => "game running"
      })) as any
    )
  ).rejects.toThrow(/game running/)
  expect(child.kill).not.toHaveBeenCalled()
})
it("failed shutdown cancels matching preparation and never kills", async () => {
  const { child, proc } = processFixture()
  const calls: string[] = []
  const transport = vi.fn(async (url, options) => {
    const endpoint = String(url).split("/").at(-1)!
    calls.push(endpoint)
    if (endpoint === "shutdown") throw new Error("transport failure")
    return {
      ok: true,
      json: async () => ({
        ...JSON.parse(options.body),
        runtimePath: process.cwd()
      })
    }
  })
  await expect(
    stopCoreForRelocation(
      { port: 1, apiToken: "fixture", process: proc },
      process.cwd(),
      transport as any
    )
  ).rejects.toThrow(/transport/)
  expect(calls).toEqual(["prepare-relocation", "shutdown", "cancel-relocation"])
  expect(child.kill).not.toHaveBeenCalled()
})
it("externally owned development core cannot authorize relocation", async () => {
  await expect(
    stopCoreForRelocation(
      { port: 1, apiToken: "fixture", process: null },
      process.cwd()
    )
  ).rejects.toThrow(/launcher-owned/)
})
