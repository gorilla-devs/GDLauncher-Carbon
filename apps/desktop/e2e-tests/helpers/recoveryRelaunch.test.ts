import fs from "node:fs"
import os from "node:os"
import path from "node:path"
import { EventEmitter } from "node:events"
import { describe, expect, it } from "vitest"
import {
  publishE2EMainStart,
  readE2EMainStart
} from "../../packages/main/utils/e2eBootstrap"
import {
  captureRelaunchAttempt,
  parseTaskkillEnded,
  stopRelaunchedMain,
  waitForRelaunchMain,
  type RelaunchWaitDeps,
  type StopRelaunchDeps
} from "./recoveryRelaunch"

function fixture() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "task43-relaunch-"))
  const expected = {
    userDataPath: path.join(root, "userData"),
    runtimePath: root,
    runId: "owned-run"
  }
  const identity = {
    pid: 10,
    executable: "/owned/app",
    startIdentity: "boot:100",
    role: "main" as const
  }
  const outgoing = Object.assign(new EventEmitter(), {
    pid: 10,
    exitCode: null,
    signalCode: null
  })
  const publish = (process = identity, extra = {}) =>
    publishE2EMainStart({
      e2eBuild: true,
      ...expected,
      observe: () => process,
      ...extra
    })!
  publish()
  const attempt = captureRelaunchAttempt(outgoing, expected)
  let now = 0
  const deps: RelaunchWaitDeps = {
    readMarker: readE2EMainStart,
    observe: () => null,
    now: () => now,
    sleep: async (ms) => {
      now += ms
    }
  }
  return { root, expected, identity, outgoing, publish, attempt, deps }
}

describe("connected main bootstrap and recovery waiter", () => {
  it("waits for outgoing exit even when a valid owned replacement marker is already present", async () => {
    const f = fixture()
    const next = { ...f.identity, pid: 11, startIdentity: "boot:200" }
    try {
      f.publish(next)
      let slept = false
      const current = await waitForRelaunchMain(f.attempt, 100, {
        ...f.deps,
        observe: () => next,
        sleep: async (ms) => {
          slept = true
          await f.deps.sleep(ms)
          f.outgoing.emit("exit", 0, null)
        }
      })
      expect(slept).toBe(true)
      expect(current).toEqual(next)
      expect(f.outgoing.listenerCount("exit")).toBe(0)
    } finally {
      fs.rmSync(f.root, { recursive: true, force: true })
    }
  })
  it("fresh foreign main surviving while original main lives never proves Restart", async () => {
    const f = fixture()
    try {
      const foreign = { ...f.identity, pid: 99, startIdentity: "boot:300" }
      f.publish(foreign, { runtimePath: path.join(f.root, "foreign") })
      await expect(
        waitForRelaunchMain(f.attempt, 100, {
          ...f.deps,
          observe: () => foreign
        })
      ).rejects.toThrow("attributable")
      expect(f.outgoing.listenerCount("exit")).toBe(0)
      expect(fs.existsSync(f.root)).toBe(true)
    } finally {
      fs.rmSync(f.root, { recursive: true, force: true })
    }
  })
  it.each([
    "unchanged",
    "helper",
    "reused",
    "unknown",
    "foreign-executable",
    "foreign-run",
    "same-pid",
    "missing"
  ])("refuses %s evidence after outgoing exit", async (mode) => {
    const f = fixture()
    const next = { ...f.identity, pid: 11, startIdentity: "boot:200" }
    try {
      f.outgoing.emit("exit", 0, null)
      if (mode !== "unchanged")
        f.publish(
          mode === "same-pid"
            ? { ...next, pid: 10 }
            : mode === "foreign-executable"
              ? { ...next, executable: "/foreign/app" }
              : next,
          mode === "foreign-run" ? { runId: "foreign" } : {}
        )
      if (mode === "missing")
        fs.rmSync(path.join(f.expected.userDataPath, "e2e-main-start.json"))
      const observe = () =>
        mode === "helper"
          ? { ...next, role: "helper" as const }
          : mode === "reused"
            ? { ...next, startIdentity: "boot:999" }
            : mode === "unknown"
              ? null
              : next
      await expect(
        waitForRelaunchMain(f.attempt, 100, { ...f.deps, observe })
      ).rejects.toThrow("attributable")
    } finally {
      fs.rmSync(f.root, { recursive: true, force: true })
    }
  })
  it("fails capture when the marker does not belong to the outgoing main", () => {
    const f = fixture()
    try {
      expect(() =>
        captureRelaunchAttempt({ ...f.outgoing, pid: 77 } as never, f.expected)
      ).toThrow("outgoing")
    } finally {
      f.attempt.dispose()
      fs.rmSync(f.root, { recursive: true, force: true })
    }
  })
  it("attributes the marker to the main pid when Playwright's child is a shell around it", async () => {
    // Windows: Playwright launches Electron through a shell, so the child it
    // hands back has the shell's pid, and the shell exits when Electron does.
    const f = fixture()
    const shell = Object.assign(new EventEmitter(), {
      pid: 4242,
      exitCode: null,
      signalCode: null
    })
    const next = { ...f.identity, pid: 11, startIdentity: "boot:200" }
    try {
      const attempt = captureRelaunchAttempt(shell, f.expected, f.identity.pid)
      f.publish(next)
      shell.emit("exit", 0, null)
      await expect(
        waitForRelaunchMain(attempt, 100, { ...f.deps, observe: () => next })
      ).resolves.toEqual(next)
      expect(shell.listenerCount("exit")).toBe(0)
    } finally {
      f.attempt.dispose()
      fs.rmSync(f.root, { recursive: true, force: true })
    }
  })
  it("fails capture when the main pid read from the app is not the marker's", () => {
    const f = fixture()
    try {
      expect(() =>
        captureRelaunchAttempt(
          { ...f.outgoing, pid: 4242 } as never,
          f.expected,
          77
        )
      ).toThrow("outgoing")
    } finally {
      f.attempt.dispose()
      fs.rmSync(f.root, { recursive: true, force: true })
    }
  })
})

describe("stopRelaunchedMain", () => {
  /** The outgoing main's marker, then (unless `relaunch` is false) a fresh
   *  one from the relaunched main, pid 11. */
  function relaunched(relaunch = true) {
    const f = fixture()
    f.attempt.dispose()
    const next = { ...f.identity, pid: 11, startIdentity: "boot:200" }
    if (relaunch) f.publish(next)
    const killedTrees: number[] = []
    const alive = new Set([11, 12, 13])
    const deps: StopRelaunchDeps = {
      readMarker: readE2EMainStart,
      observe: () => next,
      killTree: (pid) => {
        killedTrees.push(pid)
        return [12, 13, pid]
      },
      isAlive: (pid) => alive.has(pid),
      now: f.deps.now,
      sleep: async (ms) => {
        await f.deps.sleep(ms)
        alive.clear()
      }
    }
    return { ...f, next, deps, killedTrees, alive }
  }

  it("ends the attributed relaunched main with its tree and waits for it", async () => {
    const r = relaunched()
    try {
      await expect(
        stopRelaunchedMain(r.attempt.before, 1_000, r.deps)
      ).resolves.toEqual({ main: 11, killed: [11, 12, 13], survivors: [] })
      expect(r.killedTrees).toEqual([11])
    } finally {
      fs.rmSync(r.root, { recursive: true, force: true })
    }
  })

  it("reports what is still running when the wait runs out", async () => {
    const r = relaunched()
    try {
      const stopped = await stopRelaunchedMain(r.attempt.before, 300, {
        ...r.deps,
        isAlive: (pid) => pid === 13
      })
      expect(stopped?.survivors).toEqual([13])
    } finally {
      fs.rmSync(r.root, { recursive: true, force: true })
    }
  })

  it.each([
    "no-relaunch",
    "same-pid",
    "foreign-run",
    "foreign-runtime",
    "foreign-executable",
    "gone",
    "reused",
    "helper"
  ])("touches nothing on %s evidence", async (mode) => {
    const r = relaunched(mode !== "no-relaunch")
    try {
      if (mode === "same-pid") r.publish({ ...r.next, pid: 10 })
      if (mode === "foreign-run") r.publish(r.next, { runId: "foreign" })
      if (mode === "foreign-runtime")
        r.publish(r.next, { runtimePath: path.join(r.root, "foreign") })
      if (mode === "foreign-executable")
        r.publish({ ...r.next, executable: "/foreign/app" })
      const observe = () =>
        mode === "gone"
          ? null
          : mode === "reused"
            ? { ...r.next, startIdentity: "boot:999" }
            : mode === "helper"
              ? { ...r.next, role: "helper" as const }
              : r.next
      await expect(
        stopRelaunchedMain(r.attempt.before, 1_000, { ...r.deps, observe })
      ).resolves.toBeNull()
      expect(r.killedTrees).toEqual([])
    } finally {
      fs.rmSync(r.root, { recursive: true, force: true })
    }
  })
})

describe("parseTaskkillEnded", () => {
  it("reads every pid taskkill reports ended and skips its errors", () => {
    expect(
      parseTaskkillEnded(
        [
          "SUCCESS: The process with PID 5124 (child process of PID 4100) has been terminated.",
          "ERROR: The process with PID 6000 (child process of PID 4100) could not be terminated.",
          "Reason: There is no running instance of the task.",
          "SUCCESS: The process with PID 4100 (child process of PID 3000) has been terminated.",
          ""
        ].join("\r\n")
      )
    ).toEqual([5124, 4100])
  })

  it("reads nothing from empty output", () => {
    expect(parseTaskkillEnded("")).toEqual([])
  })
})
