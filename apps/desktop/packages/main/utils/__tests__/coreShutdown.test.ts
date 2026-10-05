import { it, expect, vi, afterEach } from "vitest"
import { EventEmitter } from "node:events"
import {
  ShutdownBlockedError,
  ShutdownRefusedError,
  cancelCoreShutdown,
  checkRelocation,
  forceTerminateCore,
  terminateAndObserveCoreExit,
  stopCoreForRelocation,
  stopCoreForShutdown
} from "../coreShutdown"
import { spawn, type ChildProcess } from "node:child_process"
import fs from "node:fs"
import os from "node:os"
import path from "node:path"
afterEach(() => vi.useRealTimers())
function answer(status: number, body: unknown) {
  return {
    ok: status < 300,
    status,
    json: async () => body,
    text: async () => JSON.stringify(body)
  }
}
/** A fake core: records every call and answers through `reply`. */
function fakeCore(
  reply: (endpoint: string, body: any) => ReturnType<typeof answer>
) {
  const calls: { endpoint: string; body: any }[] = []
  const transport = vi.fn(async (url: string, options: any) => {
    const endpoint = String(url).split("/").at(-1)!
    const body = JSON.parse(options.body)
    calls.push({ endpoint, body })
    return reply(endpoint, body)
  })
  return { calls, transport: transport as any }
}
const leased = (body: any) =>
  answer(200, { requestId: body.requestId, runtimePath: process.cwd() })
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
        runtimePath: process.cwd(),
        status: { ...JSON.parse(options.body), phase: "ready", blockers: [] }
      })
    }
  })
  await stopCoreForRelocation(
    { port: 1, apiToken: "synthetic-token", process: proc },
    process.cwd(),
    transport as any
  )
  // The refuse-first lease, never the freezing prepare-shutdown path.
  expect(calls).toEqual(["prepare-relocation", "shutdown", "kill"])
})
it("busy core does not receive kill", async () => {
  const { child, proc } = processFixture()
  const attempt = stopCoreForRelocation(
    { port: 1, apiToken: "fixture", process: proc },
    process.cwd(),
    vi.fn(async () => ({
      ok: false,
      status: 409,
      text: async () => "game running"
    })) as any
  )
  await expect(attempt).rejects.toThrow(/game running/)
  await expect(attempt).rejects.toBeInstanceOf(ShutdownRefusedError)
  expect(child.kill).not.toHaveBeenCalled()
})
it("a server error from the core is not taken as a refusal", async () => {
  const { child, proc } = processFixture()
  const core = fakeCore(() => answer(500, "internal error"))
  const attempt = stopCoreForRelocation(
    { port: 1, apiToken: "fixture", process: proc },
    process.cwd(),
    core.transport
  )
  await expect(attempt).rejects.toThrow(/internal error/)
  await expect(attempt).rejects.not.toBeInstanceOf(ShutdownRefusedError)
  expect(core.calls.map((call) => call.endpoint)).toEqual([
    "prepare-relocation",
    "prepare-relocation",
    "cancel-relocation"
  ])
  expect(child.kill).not.toHaveBeenCalled()
})
it("a failed commit that the core released stays dismissible and is not killed", async () => {
  const { child, proc } = processFixture()
  const calls: string[] = []
  const transport = vi.fn(async (url, options) => {
    const endpoint = String(url).split("/").at(-1)!
    calls.push(endpoint)
    if (endpoint === "shutdown") throw new Error("transport failure")
    return answer(200, {
      ...JSON.parse(options.body),
      runtimePath: process.cwd()
    })
  })
  const attempt = stopCoreForRelocation(
    { port: 1, apiToken: "fixture", process: proc },
    process.cwd(),
    transport as any
  )
  await expect(attempt).rejects.toThrow(/transport/)
  await expect(attempt).rejects.toBeInstanceOf(ShutdownRefusedError)
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

it("lost preparation and commit ACKs recover the same request before observed exit", async () => {
  const { child, proc } = processFixture()
  const ids: string[] = []
  let prepares = 0
  child.kill.mockImplementation(() => {
    child.emit("exit", 0)
    return true
  })
  const transport = vi.fn(async (url, options) => {
    const body = JSON.parse(options.body)
    ids.push(body.requestId)
    const endpoint = String(url).split("/").at(-1)
    if (endpoint === "prepare-shutdown" && ++prepares === 1)
      throw new Error("lost prepare ACK")
    if (endpoint === "shutdown") throw new Error("lost commit ACK")
    return {
      ok: true,
      json: async () => ({
        ...body,
        runtimePath: process.cwd(),
        status: {
          ...body,
          phase: endpoint === "shutdown-status" ? "committed" : "ready",
          blockers: []
        }
      })
    }
  })
  await stopCoreForShutdown(
    { port: 1, apiToken: "fixture", process: proc },
    "quit",
    transport as any
  )
  expect(new Set(ids).size).toBe(1)
  expect(child.kill).toHaveBeenCalledOnce()
})
it("installer pending outcome remains visible and retry keeps its ID", async () => {
  const { child, proc } = processFixture()
  const ids: string[] = []
  const transport = vi.fn(async (_url, options) => {
    const body = JSON.parse(options.body)
    ids.push(body.requestId)
    return {
      ok: true,
      json: async () => ({
        ...body,
        status: {
          ...body,
          phase: "unresolved",
          blockers: ["Installer participant pending Task18"]
        }
      })
    }
  })
  for (let attempt = 0; attempt < 2; attempt++)
    await expect(
      stopCoreForShutdown(
        { port: 1, apiToken: "fixture", process: proc },
        "quit",
        transport as any
      )
    ).rejects.toThrow(/Task18/)
  expect(new Set(ids).size).toBe(1)
  expect(child.kill).not.toHaveBeenCalled()
})
it("waits out a 40 s server drain and quits on the first attempt", async () => {
  // The core's worst case is about 42 s: 30 s for the server to save after
  // 'stop', then a kill, then its last log lines. That must fit the deadline.
  vi.useFakeTimers()
  const { child, proc } = processFixture()
  child.kill.mockImplementation(() => {
    child.emit("exit", 0)
    return true
  })
  const calls: string[] = []
  const startedAt = Date.now()
  const transport = vi.fn(async (url, options) => {
    const body = JSON.parse(options.body)
    const endpoint = String(url).split("/").at(-1)!
    calls.push(endpoint)
    const phase =
      endpoint === "shutdown-status" && Date.now() - startedAt >= 40000
        ? "ready"
        : "draining"
    return {
      ok: true,
      json: async () => ({
        ...body,
        runtimePath: process.cwd(),
        status: { ...body, phase, blockers: [] }
      })
    }
  })
  const quit = expect(
    stopCoreForShutdown(
      { port: 1, apiToken: "fixture", process: proc },
      "quit",
      transport as any
    )
  ).resolves.toBeUndefined()
  await vi.advanceTimersByTimeAsync(41000)
  await quit
  expect(calls[0]).toBe("prepare-shutdown")
  expect(calls.filter((call) => call === "prepare-shutdown")).toHaveLength(1)
  expect(calls.at(-1)).toBe("shutdown")
  expect(child.kill).toHaveBeenCalledOnce()
})

it("waits out a 42 s server drain followed by the hand-over of running games", async () => {
  // After the drain the core cancels the session workers of running games
  // and gives them up to 5 s to hand their games over, so its worst case is
  // about 47 s. Cutting it off earlier would ask the user about a quit that
  // was about to finish.
  vi.useFakeTimers()
  const { child, proc } = processFixture()
  child.kill.mockImplementation(() => {
    child.emit("exit", 0)
    return true
  })
  const startedAt = Date.now()
  const transport = vi.fn(async (url, options) => {
    const body = JSON.parse(options.body)
    const endpoint = String(url).split("/").at(-1)!
    const phase =
      endpoint === "shutdown-status" && Date.now() - startedAt >= 47000
        ? "ready"
        : "draining"
    return {
      ok: true,
      json: async () => ({
        ...body,
        runtimePath: process.cwd(),
        status: { ...body, phase, blockers: [], serversStopped: true }
      })
    }
  })
  const quit = expect(
    stopCoreForShutdown(
      { port: 1, apiToken: "fixture", process: proc },
      "quit",
      transport as any
    )
  ).resolves.toBeUndefined()
  await vi.advanceTimersByTimeAsync(48000)
  await quit
  expect(child.kill).toHaveBeenCalledOnce()
})

it("a core that exits while it drains ends the wait at once instead of at the deadline", async () => {
  // A deletion keeps the quit waiting; then the core crashes or is ended in
  // Task Manager. Its port refuses every later request.
  vi.useFakeTimers()
  const { child, proc } = processFixture()
  const startedAt = Date.now()
  const transport = vi.fn(async (url: string, options: any) => {
    const body = JSON.parse(options.body)
    if (Date.now() - startedAt >= 300)
      throw new TypeError("fetch failed", {
        cause: Object.assign(new Error("connect ECONNREFUSED"), {
          code: "ECONNREFUSED"
        })
      })
    return answer(200, {
      ...body,
      runtimePath: process.cwd(),
      status: {
        ...body,
        phase: "draining",
        blockers: [{ kind: "deletion", name: "delete instance 3" }]
      }
    })
  })
  setTimeout(() => {
    Object.assign(child, { exitCode: 1 })
    child.emit("exit", 1, null)
  }, 300)
  const statuses: string[] = []
  let settledAt: number | undefined
  const quit = stopCoreForShutdown(
    { port: 1, apiToken: "fixture", process: proc },
    "quit",
    transport as any,
    5000,
    { onStatus: (status) => statuses.push(status.phase) }
  ).then(() => {
    settledAt = Date.now() - startedAt
  })
  await vi.advanceTimersByTimeAsync(1000)
  await quit
  expect(settledAt).toBeLessThan(1000)
  expect(statuses[0]).toBe("draining")
  // Nothing is left to signal, and nothing is held for a later attempt.
  expect(child.kill).not.toHaveBeenCalled()
  const sent = transport.mock.calls.length
  await expect(
    cancelCoreShutdown(
      { port: 1, apiToken: "fixture", process: proc },
      transport as any
    )
  ).resolves.toBe("released")
  expect(transport.mock.calls.length).toBe(sent)
})

it("choosing to stop now or keep GDLauncher open does not wait for a core that is slow to answer", async () => {
  const { child, proc } = processFixture()
  const sent: string[] = []
  // A core that never answers: only the request's own signal ends a call.
  const transport = vi.fn(
    (url: string, options: any) =>
      new Promise((_resolve, reject) => {
        const signal: AbortSignal = options.signal
        const aborted = () =>
          reject(new Error("aborted", { cause: signal.reason }))
        if (signal.aborted) return aborted()
        sent.push(String(url).split("/").at(-1)!)
        signal.addEventListener("abort", aborted)
      })
  )
  const choice = new AbortController()
  const stop = stopCoreForShutdown(
    { port: 1, apiToken: "fixture", process: proc },
    "quit",
    transport as any,
    5000,
    { signal: choice.signal }
  ).catch((error) => error)
  await new Promise((resolve) => setTimeout(resolve, 10))
  const chosenAt = Date.now()
  choice.abort()
  const error = await stop
  expect(Date.now() - chosenAt).toBeLessThan(1000)
  expect(error).toBeInstanceOf(ShutdownBlockedError)
  expect(error.reason).toBe("unreachable")
  // No second request goes out once the user has chosen.
  expect(sent).toEqual(["prepare-shutdown"])
  expect(child.kill).not.toHaveBeenCalled()
})

it("a refused move throws its blockers and leaves quit free to start its own request", async () => {
  const { child, proc } = processFixture()
  const core = { port: 1, apiToken: "fixture", process: proc }
  const blockers = [{ kind: "game", name: "game 3" }]
  const relocation = fakeCore((endpoint) =>
    endpoint === "prepare-relocation"
      ? answer(409, { error: "Runtime is busy", blockers })
      : answer(500, "unexpected")
  )
  const refusal = await stopCoreForRelocation(
    core,
    process.cwd(),
    relocation.transport
  ).catch((error) => error)
  expect(refusal).toBeInstanceOf(ShutdownRefusedError)
  expect(refusal.blockers).toEqual(blockers)
  expect(relocation.calls.map((call) => call.endpoint)).toEqual([
    "prepare-relocation"
  ])
  expect(child.kill).not.toHaveBeenCalled()

  child.kill.mockImplementation(() => {
    child.emit("exit", 0)
    return true
  })
  const quit = fakeCore((_, body) =>
    answer(200, { ...body, status: { ...body, phase: "ready", blockers: [] } })
  )
  await stopCoreForShutdown(core, "quit", quit.transport)
  expect(quit.calls[0]).toMatchObject({
    endpoint: "prepare-shutdown",
    body: { purpose: "quit" }
  })
  expect(quit.calls[0].body.requestId).not.toBe(
    relocation.calls[0].body.requestId
  )
  expect(child.kill).toHaveBeenCalledOnce()
})
/**
 * A fake core that, like runtime_lifecycle.rs begin_relocation, retires every
 * id it refused or cancelled: asking again under one is refused as cancelled.
 */
function retiringCore(busy: () => unknown[], lost: (call: number) => boolean) {
  const retired = new Set<string>()
  let calls = 0
  return fakeCore((endpoint, body) => {
    const call = ++calls
    let reply
    if (endpoint === "prepare-relocation") {
      if (retired.has(body.requestId))
        reply = answer(409, { error: "refused", blockers: [], cancelled: true })
      else if (busy().length) {
        retired.add(body.requestId)
        reply = answer(409, {
          error: "busy",
          blockers: busy(),
          cancelled: false
        })
      } else reply = leased(body)
    } else if (endpoint === "shutdown")
      reply = answer(200, { requestId: body.requestId })
    else reply = answer(500, "unexpected")
    if (lost(call)) throw new Error("The operation was aborted due to timeout")
    return reply
  })
}
it("a refusal whose answer was lost is asked again under a new id and still names what runs", async () => {
  const { child, proc } = processFixture()
  const blockers = [{ kind: "install", name: "addon installation tree" }]
  // The core refuses the first prepare, but its answer never arrives.
  const core = retiringCore(
    () => blockers,
    (call) => call === 1
  )
  const refusal = await stopCoreForRelocation(
    { port: 1, apiToken: "fixture", process: proc },
    process.cwd(),
    core.transport
  ).catch((error) => error)
  expect(refusal).toBeInstanceOf(ShutdownRefusedError)
  expect(refusal.blockers).toEqual(blockers)
  const ids = core.calls.map((call) => call.body.requestId)
  expect(core.calls.map((call) => call.endpoint)).toEqual([
    "prepare-relocation",
    "prepare-relocation",
    "prepare-relocation"
  ])
  // The retry re-asks under the same id, finds it retired, and starts over.
  expect(ids[1]).toBe(ids[0])
  expect(ids[2]).not.toBe(ids[0])
  expect(child.kill).not.toHaveBeenCalled()
})
it("a refused id is never asked again, so a late duplicate of it finds it retired", async () => {
  const { child, proc } = processFixture()
  const relocation = { port: 1, apiToken: "fixture", process: proc }
  let busy = [{ kind: "install", name: "addon installation tree" }]
  // The first prepare stalls; the retry with the same id is refused.
  const core = retiringCore(
    () => busy,
    (call) => call === 1
  )
  const refusal = await stopCoreForRelocation(
    relocation,
    process.cwd(),
    core.transport
  ).catch((error) => error)
  expect(refusal).toBeInstanceOf(ShutdownRefusedError)
  const refusedId = core.calls[1].body.requestId
  expect(core.calls[0].body.requestId).toBe(refusedId)

  // The download finishes and the user presses Try again.
  busy = []
  child.kill.mockImplementation(() => {
    child.emit("exit", 0)
    return true
  })
  await stopCoreForRelocation(relocation, process.cwd(), core.transport)
  const later = core.calls.slice(2).map((call) => call.body.requestId)
  expect(later).not.toContain(refusedId)
  expect(child.kill).toHaveBeenCalledOnce()
})
it("a core on another runtime folder gets its lease cancelled and is not killed", async () => {
  const { child, proc } = processFixture()
  const core = fakeCore((endpoint, body) =>
    endpoint === "prepare-relocation"
      ? answer(200, { requestId: body.requestId, runtimePath: "/elsewhere" })
      : answer(200, { requestId: body.requestId })
  )
  await expect(
    stopCoreForRelocation(
      { port: 1, apiToken: "fixture", process: proc },
      process.cwd(),
      core.transport
    )
  ).rejects.toBeInstanceOf(ShutdownRefusedError)
  expect(core.calls.map((call) => call.endpoint)).toEqual([
    "prepare-relocation",
    "cancel-relocation"
  ])
  expect(child.kill).not.toHaveBeenCalled()
})
it("a lost commit ACK whose lease the core reports committed goes on to stop the core", async () => {
  const { child, proc } = processFixture()
  child.kill.mockImplementation(() => {
    child.emit("exit", 0)
    return true
  })
  const core = fakeCore((endpoint, body) => {
    if (endpoint === "shutdown") throw new Error("lost commit ACK")
    if (endpoint === "cancel-relocation")
      return answer(409, {
        error: "Relocation is already committed",
        committed: true
      })
    return leased(body)
  })
  await stopCoreForRelocation(
    { port: 1, apiToken: "fixture", process: proc },
    process.cwd(),
    core.transport
  )
  expect(core.calls.map((call) => call.endpoint)).toEqual([
    "prepare-relocation",
    "shutdown",
    "cancel-relocation"
  ])
  expect(child.kill).toHaveBeenCalledOnce()
})
it("a commit refused because no lease is left never stops the core", async () => {
  const { child, proc } = processFixture()
  const core = fakeCore((endpoint, body) => {
    if (endpoint === "shutdown")
      return answer(409, "Shutdown requires the matching relocation lease")
    if (endpoint === "cancel-relocation")
      return answer(409, {
        error: "No matching cancellable relocation",
        committed: false
      })
    return leased(body)
  })
  await expect(
    stopCoreForRelocation(
      { port: 1, apiToken: "fixture", process: proc },
      process.cwd(),
      core.transport
    )
  ).rejects.toBeInstanceOf(ShutdownRefusedError)
  expect(child.kill).not.toHaveBeenCalled()
})
it("a retry after an unobserved exit re-joins the same lease", async () => {
  const { child, proc } = processFixture()
  const core = fakeCore((_, body) => leased(body))
  const relocation = { port: 1, apiToken: "fixture", process: proc }
  // The core got the signal but its exit was not seen in time.
  await expect(
    stopCoreForRelocation(relocation, process.cwd(), core.transport, 20)
  ).rejects.toThrow(/not confirmed/)
  child.kill.mockImplementation(() => {
    child.emit("exit", 0)
    return true
  })
  await stopCoreForRelocation(relocation, process.cwd(), core.transport, 20)
  const ids = core.calls.map((call) => call.body.requestId)
  expect(new Set(ids).size).toBe(1)
  expect(core.calls.map((call) => call.endpoint)).toEqual([
    "prepare-relocation",
    "shutdown",
    "prepare-relocation",
    "shutdown"
  ])
})
it("an unanswered release keeps the lease id so a retry can finish or cancel it", async () => {
  const { child, proc } = processFixture()
  const relocation = { port: 1, apiToken: "fixture", process: proc }
  let unreachable = true
  const core = fakeCore((endpoint, body) => {
    if (unreachable && endpoint !== "prepare-relocation")
      throw new Error("core unreachable")
    return leased(body)
  })
  const first = await stopCoreForRelocation(
    relocation,
    process.cwd(),
    core.transport
  ).catch((error) => error)
  // Not a refusal: the core may still hold the lease, so it is not dismissible.
  expect(first).not.toBeInstanceOf(ShutdownRefusedError)
  expect(String(first)).toMatch(/unreachable/)
  unreachable = false
  child.kill.mockImplementation(() => {
    child.emit("exit", 0)
    return true
  })
  await stopCoreForRelocation(relocation, process.cwd(), core.transport)
  expect(new Set(core.calls.map((call) => call.body.requestId)).size).toBe(1)
})
it("a lost lease ACK is retried with the same id", async () => {
  const { child, proc } = processFixture()
  child.kill.mockImplementation(() => {
    child.emit("exit", 0)
    return true
  })
  let prepares = 0
  const core = fakeCore((endpoint, body) => {
    if (endpoint === "prepare-relocation" && ++prepares === 1)
      throw new Error("lost lease ACK")
    return leased(body)
  })
  await stopCoreForRelocation(
    { port: 1, apiToken: "fixture", process: proc },
    process.cwd(),
    core.transport
  )
  expect(core.calls.map((call) => call.endpoint)).toEqual([
    "prepare-relocation",
    "prepare-relocation",
    "shutdown"
  ])
  expect(new Set(core.calls.map((call) => call.body.requestId)).size).toBe(1)
})
it("a move whose lease requests went unanswered and whose cancel found nothing starts the next attempt under a new id", async () => {
  // The core cancelled the id, so a prepare it only gets to now is refused
  // there instead of granting a lease nobody owns (runtime.rs tests).
  const { child, proc } = processFixture()
  const relocation = { port: 1, apiToken: "fixture", process: proc }
  let stalled = true
  const core = fakeCore((endpoint, body) => {
    if (endpoint === "prepare-relocation" && stalled)
      throw new Error("The operation was aborted due to timeout")
    if (endpoint === "cancel-relocation")
      return answer(409, {
        error: "No matching cancellable relocation",
        committed: false
      })
    return leased(body)
  })
  const first = await stopCoreForRelocation(
    relocation,
    process.cwd(),
    core.transport
  ).catch((error) => error)
  expect(first).toBeInstanceOf(ShutdownRefusedError)
  expect(core.calls.map((call) => call.endpoint)).toEqual([
    "prepare-relocation",
    "prepare-relocation",
    "cancel-relocation"
  ])
  expect(child.kill).not.toHaveBeenCalled()

  stalled = false
  child.kill.mockImplementation(() => {
    child.emit("exit", 0)
    return true
  })
  await stopCoreForRelocation(relocation, process.cwd(), core.transport)
  const [cancelled, next] = [core.calls[2], core.calls[3]]
  expect(next.endpoint).toBe("prepare-relocation")
  expect(next.body.requestId).not.toBe(cancelled.body.requestId)
})
it("a retry whose id the core already cancelled starts over under a new id and finishes the move", async () => {
  const { child, proc } = processFixture()
  const relocation = { port: 1, apiToken: "fixture", process: proc }
  const cancelled = new Set<string>()
  let unreachable = true
  const core = fakeCore((endpoint, body) => {
    if (endpoint === "prepare-relocation" && cancelled.has(body.requestId))
      return answer(409, {
        error: "This move attempt was already cancelled; start a new one",
        blockers: [],
        cancelled: true
      })
    if (endpoint === "shutdown" && unreachable)
      throw new Error("lost commit ACK")
    if (endpoint === "cancel-relocation") {
      // The release lands, but its answer is lost on the way back.
      cancelled.add(body.requestId)
      throw new Error("lost cancel ACK")
    }
    return leased(body)
  })
  const first = await stopCoreForRelocation(
    relocation,
    process.cwd(),
    core.transport
  ).catch((error) => error)
  // Not known to be released, so not dismissible and the id is kept.
  expect(first).not.toBeInstanceOf(ShutdownRefusedError)

  unreachable = false
  child.kill.mockImplementation(() => {
    child.emit("exit", 0)
    return true
  })
  await stopCoreForRelocation(relocation, process.cwd(), core.transport)
  expect(core.calls.map((call) => call.endpoint)).toEqual([
    "prepare-relocation",
    "shutdown",
    "cancel-relocation",
    "prepare-relocation",
    "prepare-relocation",
    "shutdown"
  ])
  const ids = core.calls.map((call) => call.body.requestId)
  expect(ids[3]).toBe(ids[0])
  expect(ids[4]).not.toBe(ids[0])
  expect(ids[5]).toBe(ids[4])
  expect(child.kill).toHaveBeenCalledOnce()
})

it("the blocker check before the folder scan refuses with what runs and takes no lease", async () => {
  const { child, proc } = processFixture()
  const blockers = [{ kind: "server", name: "server 7 run" }]
  const core = fakeCore(() => answer(200, { blockers }))
  const refusal = await checkRelocation(
    { port: 1, apiToken: "fixture", process: proc },
    core.transport
  ).catch((error) => error)
  expect(refusal).toBeInstanceOf(ShutdownRefusedError)
  expect(refusal.blockers).toEqual(blockers)
  expect(core.calls.map((call) => call.endpoint)).toEqual([
    "relocation-blockers"
  ])
  expect(child.kill).not.toHaveBeenCalled()
})
it("the blocker check lets the move go on when nothing runs or the core does not answer", async () => {
  const { proc } = processFixture()
  const core = { port: 1, apiToken: "fixture", process: proc }
  await expect(
    checkRelocation(
      core,
      fakeCore(() => answer(200, { blockers: [] })).transport
    )
  ).resolves.toBeUndefined()
  // prepare-relocation asks again and decides; the check only saves time.
  await expect(
    checkRelocation(
      core,
      fakeCore(() => {
        throw new Error("The operation was aborted due to timeout")
      }).transport
    )
  ).resolves.toBeUndefined()
  await expect(
    checkRelocation(core, fakeCore(() => answer(500, "internal")).transport)
  ).resolves.toBeUndefined()
})
it("the blocker check names an earlier attempt's lease so it is not counted against the retry", async () => {
  const { proc } = processFixture()
  const relocation = { port: 1, apiToken: "fixture", process: proc }
  const core = fakeCore((endpoint, body) =>
    endpoint === "relocation-blockers"
      ? answer(200, { blockers: [] })
      : leased(body)
  )
  // The commit landed but the exit was not seen in time.
  await expect(
    stopCoreForRelocation(relocation, process.cwd(), core.transport, 20)
  ).rejects.toThrow(/not confirmed/)
  await checkRelocation(relocation, core.transport)
  const check = core.calls.at(-1)!
  expect(check.endpoint).toBe("relocation-blockers")
  expect(check.body.requestId).toBe(core.calls[0].body.requestId)
})
it("the blocker check refuses a move without a launcher-owned core before any scan", async () => {
  await expect(
    checkRelocation({ port: 1, apiToken: "fixture", process: null })
  ).rejects.toBeInstanceOf(ShutdownRefusedError)
})

/** A core that answers every shutdown request with `phase`. */
const phaseCore = (phase: () => string, blockers: unknown[] = []) =>
  fakeCore((endpoint, body) => {
    if (endpoint === "cancel-shutdown")
      return answer(200, { requestId: body.requestId })
    return answer(200, {
      requestId: body.requestId,
      status: { ...body, phase: phase(), blockers, serversStopped: false }
    })
  })
it("a refusal before the freeze forgets the request, so a later update starts its own", async () => {
  const { proc } = processFixture()
  const core = { port: 1, apiToken: "fixture", process: proc }
  const refusing = fakeCore(() =>
    answer(409, "Another request owns runtime shutdown")
  )
  const refusal = await stopCoreForShutdown(
    core,
    "quit",
    refusing.transport
  ).catch((error) => error)
  expect(refusal).toBeInstanceOf(ShutdownBlockedError)
  expect(refusal.reason).toBe("busy")
  // A definite refusal is not retried under the same id.
  expect(refusing.calls.map((call) => call.endpoint)).toEqual([
    "prepare-shutdown"
  ])

  const update = phaseCore(() => "unresolved")
  await stopCoreForShutdown(core, "update", update.transport).catch(() => {})
  expect(update.calls[0]).toMatchObject({
    endpoint: "prepare-shutdown",
    body: { purpose: "update" }
  })
  expect(update.calls[0].body.requestId).not.toBe(
    refusing.calls[0].body.requestId
  )
})
it("an unresolved attempt is retried under the same request id and names its blockers", async () => {
  const { child, proc } = processFixture()
  const core = { port: 1, apiToken: "fixture", process: proc }
  const blockers = [{ kind: "deletion", name: "delete instance 12" }]
  const unresolved = phaseCore(() => "unresolved", blockers)
  const first = await stopCoreForShutdown(
    core,
    "quit",
    unresolved.transport
  ).catch((error) => error)
  expect(first).toBeInstanceOf(ShutdownBlockedError)
  expect(first.reason).toBe("unresolved")
  expect(first.blockers).toEqual(blockers)
  await stopCoreForShutdown(core, "quit", unresolved.transport).catch(() => {})
  const ids = unresolved.calls.map((call) => call.body.requestId)
  expect(new Set(ids).size).toBe(1)
  expect(child.kill).not.toHaveBeenCalled()
})
it("an update after an unresolved quit replaces it instead of throwing, and one after a draining quit joins it", async () => {
  const { child, proc } = processFixture()
  child.kill.mockImplementation(() => {
    child.emit("exit", 0)
    return true
  })
  const core = { port: 1, apiToken: "fixture", process: proc }
  const quit = phaseCore(() => "unresolved")
  await stopCoreForShutdown(core, "quit", quit.transport).catch(() => {})
  const update = phaseCore(() => "ready")
  await stopCoreForShutdown(core, "update", update.transport)
  expect(update.calls.map((call) => call.endpoint)).toEqual([
    "prepare-shutdown",
    "shutdown"
  ])
  expect(update.calls[0].body.purpose).toBe("update")
  expect(update.calls[0].body.requestId).not.toBe(quit.calls[0].body.requestId)
  expect(child.kill).toHaveBeenCalledOnce()

  // A quit that was still draining when the launcher stopped waiting.
  const second = processFixture()
  const other = { port: 1, apiToken: "fixture", process: second.proc }
  const draining = phaseCore(() => "draining")
  const stopped = new AbortController()
  stopped.abort()
  const slow = await stopCoreForShutdown(
    other,
    "quit",
    draining.transport,
    5000,
    {
      signal: stopped.signal
    }
  ).catch((error) => error)
  expect(slow.reason).toBe("slow")
  const joined = phaseCore(() => "unresolved")
  await stopCoreForShutdown(other, "update", joined.transport).catch(() => {})
  expect(joined.calls[0].body).toEqual(draining.calls[0].body)
  expect(joined.calls[0].body.purpose).toBe("quit")
})
it("Keep GDLauncher open hands the request back, and the next attempt starts a new one", async () => {
  const { proc } = processFixture()
  const core = { port: 1, apiToken: "fixture", process: proc }
  const unresolved = phaseCore(() => "unresolved")
  await stopCoreForShutdown(core, "quit", unresolved.transport).catch(() => {})
  expect(await cancelCoreShutdown(core, unresolved.transport)).toBe("released")
  const cancel = unresolved.calls.at(-1)!
  expect(cancel.endpoint).toBe("cancel-shutdown")
  expect(cancel.body).toEqual({ requestId: unresolved.calls[0].body.requestId })
  await stopCoreForShutdown(core, "quit", unresolved.transport).catch(() => {})
  expect(unresolved.calls.at(-1)!.body.requestId).not.toBe(
    cancel.body.requestId
  )
  // Nothing held: nothing to send.
  const idle = processFixture()
  const nothing = fakeCore(() => answer(500, "unexpected"))
  expect(
    await cancelCoreShutdown(
      { port: 1, apiToken: "fixture", process: idle.proc },
      nothing.transport
    )
  ).toBe("released")
  expect(nothing.calls).toEqual([])
})
it("a request the core can no longer give back keeps it, so the stop is finished instead", async () => {
  const { proc } = processFixture()
  const core = { port: 1, apiToken: "fixture", process: proc }
  const unresolved = phaseCore(() => "unresolved")
  await stopCoreForShutdown(core, "quit", unresolved.transport).catch(() => {})
  const finishing = fakeCore(() =>
    answer(409, { error: "Shutdown is already finishing", finishing: true })
  )
  expect(await cancelCoreShutdown(core, finishing.transport)).toBe("finishing")
  await stopCoreForShutdown(core, "quit", unresolved.transport).catch(() => {})
  expect(
    new Set(unresolved.calls.map((call) => call.body.requestId)).size
  ).toBe(1)
  // An unanswered cancel keeps it too.
  const lost = fakeCore(() => {
    throw new Error("The operation was aborted due to timeout")
  })
  await expect(cancelCoreShutdown(core, lost.transport)).rejects.toThrow()
  await stopCoreForShutdown(core, "quit", unresolved.transport).catch(() => {})
  expect(
    new Set(unresolved.calls.map((call) => call.body.requestId)).size
  ).toBe(1)
})
it("a Keep GDLauncher open that landed unanswered does not show the next quit a refusal: it starts over under a new id", async () => {
  const { child, proc } = processFixture()
  child.kill.mockImplementation(() => {
    child.emit("exit", 0)
    return true
  })
  const core = { port: 1, apiToken: "fixture", process: proc }
  const unresolved = phaseCore(() => "unresolved")
  await stopCoreForShutdown(core, "quit", unresolved.transport).catch(() => {})
  const first = unresolved.calls[0].body.requestId
  // The core cancelled and retired the request; its answer was lost.
  const lost = fakeCore(() => {
    throw new Error("The operation was aborted due to timeout")
  })
  await expect(cancelCoreShutdown(core, lost.transport)).rejects.toThrow()
  const retired = new Set([first])
  const core2 = fakeCore((endpoint, body) => {
    if (retired.has(body.requestId))
      return answer(409, {
        error: "This shutdown request was cancelled or replaced",
        cancelled: true
      })
    return answer(200, {
      requestId: body.requestId,
      status: { ...body, phase: "ready", blockers: [], serversStopped: false }
    })
  })
  await stopCoreForShutdown(core, "quit", core2.transport)
  expect(
    core2.calls.map((call) => [call.endpoint, call.body.requestId])
  ).toEqual([
    ["prepare-shutdown", first],
    ["prepare-shutdown", expect.not.stringMatching(first)],
    ["shutdown", core2.calls[1].body.requestId]
  ])
  expect(child.kill).toHaveBeenCalledOnce()

  // Any other refusal is still the user's to see.
  const other = processFixture()
  const busy = fakeCore(() =>
    answer(409, {
      error: "Another request owns runtime shutdown",
      cancelled: false
    })
  )
  const refusal = await stopCoreForShutdown(
    { port: 1, apiToken: "fixture", process: other.proc },
    "quit",
    busy.transport
  ).catch((error) => error)
  expect(refusal.reason).toBe("busy")
  expect(refusal.message).toContain("Another request owns runtime shutdown")
  expect(busy.calls).toHaveLength(1)
})
it("forceTerminateCore escalates to SIGKILL when the core ignores SIGTERM and resolves only on exit", async () => {
  vi.useFakeTimers()
  const { child, proc } = processFixture()
  const signals: string[] = []
  child.kill.mockImplementation(((signal: string) => {
    signals.push(signal)
    return true
  }) as any)
  let resolved = false
  const forced = forceTerminateCore(proc, 3000, 5000).then(() => {
    resolved = true
  })
  expect(signals).toEqual(["SIGTERM"])
  await vi.advanceTimersByTimeAsync(2999)
  expect(signals).toEqual(["SIGTERM"])
  await vi.advanceTimersByTimeAsync(1)
  expect(signals).toEqual(["SIGTERM", "SIGKILL"])
  await vi.advanceTimersByTimeAsync(1000)
  expect(resolved).toBe(false)
  ;(child as any).signalCode = "SIGKILL"
  child.emit("exit", null, "SIGKILL")
  await forced
  expect(resolved).toBe(true)
  expect(child.listenerCount("exit")).toBe(0)
  expect(child.listenerCount("error")).toBe(0)
})
it("forceTerminateCore stops at SIGTERM when the core exits on it, and fails if even SIGKILL is not seen to end it", async () => {
  vi.useFakeTimers()
  const quick = processFixture()
  quick.child.kill.mockImplementation(() => {
    quick.child.emit("exit", 0)
    return true
  })
  await forceTerminateCore(quick.proc)
  expect(quick.child.kill).toHaveBeenCalledOnce()

  const stuck = processFixture()
  const pending = expect(
    forceTerminateCore(stuck.proc, 3000, 5000)
  ).rejects.toThrow(/not confirmed/)
  await vi.advanceTimersByTimeAsync(8000)
  await pending
  expect(stuck.child.kill).toHaveBeenCalledTimes(2)
})

/** Whether a process with `pid` exists (signal 0 only checks). */
function alive(pid: number) {
  try {
    process.kill(pid, 0)
    return true
  } catch {
    return false
  }
}
// On Windows SIGTERM already ends the process (TerminateProcess), which
// never reaches its children either.
it.skipIf(process.platform === "win32")(
  "Quit now ends a real core that refuses SIGTERM, and a server it started keeps saving and exits on its own",
  async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "gdl-forced-quit-"))
    const saved = path.join(dir, "saved")
    // The core: its SIGTERM handler goes through the shutdown handshake,
    // which a pending quit refuses, so SIGTERM changes nothing. It started
    // a server, with piped console, that is still saving after 'stop'.
    const server = [
      "process.on('SIGTERM', () => {})",
      "setTimeout(() => require('node:fs').writeFileSync(process.argv[1], 'saved'), 1500)"
    ].join(";")
    const core = spawn(
      process.execPath,
      [
        "-e",
        [
          "process.on('SIGTERM', () => {})",
          "const child = require('node:child_process').spawn(process.execPath, ['-e', process.argv[1], process.argv[2]], { stdio: 'pipe' })",
          "console.log(child.pid)",
          "setInterval(() => {}, 1000)"
        ].join(";"),
        server,
        saved
      ],
      { stdio: ["ignore", "pipe", "ignore"] }
    )
    let serverPid = 0
    try {
      serverPid = await new Promise<number>((resolve, reject) => {
        core.stdout.once("data", (data) => resolve(Number(String(data))))
        core.once("exit", () => reject(new Error("the core exited early")))
      })
      const started = Date.now()
      await forceTerminateCore(core, 300, 5000)
      expect(core.signalCode).toBe("SIGKILL")
      expect(Date.now() - started).toBeGreaterThanOrEqual(300)
      // The server outlives the core, still saving...
      expect(alive(serverPid)).toBe(true)
      expect(fs.existsSync(saved)).toBe(false)
      // ...and finishes on its own.
      await vi.waitFor(() => expect(fs.existsSync(saved)).toBe(true), {
        timeout: 10000,
        interval: 50
      })
    } finally {
      if (core.exitCode === null && core.signalCode === null)
        core.kill("SIGKILL")
      if (serverPid && alive(serverPid)) process.kill(serverPid, "SIGKILL")
      fs.rmSync(dir, { recursive: true, force: true })
    }
  },
  20000
)
