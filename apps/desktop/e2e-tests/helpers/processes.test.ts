import { spawn } from "node:child_process"
import fs from "node:fs"
import os from "node:os"
import path from "node:path"
import { afterEach, describe, expect, it } from "vitest"
import { isPidAlive, killGameProcesses } from "./processes.js"

const spawned: ReturnType<typeof spawn>[] = []
const runtimePaths: string[] = []

/**
 * A child whose command line carries `marker`, standing in for the game JVM:
 * what identifies it is the runtime path in its arguments, not where the
 * binary it runs happens to live.
 */
function spawnChildMarkedWith(marker: string) {
  const child = spawn(
    process.execPath,
    ["-e", "setTimeout(() => {}, 30000)", marker],
    { stdio: "ignore" }
  )
  spawned.push(child)
  return child
}

function runtimeWithPidfile(pid: number) {
  const runtimePath = fs.mkdtempSync(path.join(os.tmpdir(), "gdl-sweep-"))
  runtimePaths.push(runtimePath)
  const instanceDir = path.join(runtimePath, "instances", "an-instance")
  fs.mkdirSync(instanceDir, { recursive: true })
  fs.writeFileSync(
    path.join(instanceDir, ".gdl_instance.pid"),
    `${pid}\n1700000000`
  )
  return runtimePath
}

afterEach(() => {
  for (const child of spawned.splice(0)) {
    try {
      child.kill("SIGKILL")
    } catch {
      // Already gone.
    }
  }
  for (const dir of runtimePaths.splice(0)) {
    fs.rmSync(dir, { recursive: true, force: true })
  }
})

/**
 * These spawn real processes and read the OS process table, so they carry
 * fixed waits and a PowerShell round-trip on Windows. Vitest's 5s default is
 * sized for pure logic and left roughly 3s of headroom locally — not enough on
 * a loaded CI runner, where this timed out.
 */
const PROCESS_TEST_TIMEOUT = 30_000

describe("killGameProcesses", () => {
  it(
    "kills a recorded process that belongs to this runtime path",
    async () => {
      const runtimePath = fs.mkdtempSync(path.join(os.tmpdir(), "gdl-owner-"))
      runtimePaths.push(runtimePath)
      const child = spawnChildMarkedWith(runtimePath)
      await new Promise((resolve) => setTimeout(resolve, 1500))

      const instanceDir = path.join(runtimePath, "instances", "an-instance")
      fs.mkdirSync(instanceDir, { recursive: true })
      fs.writeFileSync(
        path.join(instanceDir, ".gdl_instance.pid"),
        `${child.pid}\n1700000000`
      )

      expect(killGameProcesses(runtimePath)).toEqual([child.pid])
      await new Promise((resolve) => setTimeout(resolve, 500))
      expect(isPidAlive(child.pid!)).toBe(false)
    },
    PROCESS_TEST_TIMEOUT
  )

  it(
    "leaves a recorded pid alone when it is not this run's process",
    async () => {
      const child = spawnChildMarkedWith("an-unrelated-marker")
      await new Promise((resolve) => setTimeout(resolve, 1500))
      const runtimePath = runtimeWithPidfile(child.pid!)

      expect(killGameProcesses(runtimePath)).toEqual([])
      expect(isPidAlive(child.pid!)).toBe(true)
    },
    PROCESS_TEST_TIMEOUT
  )

  it("reports nothing when there are no pidfiles", () => {
    const runtimePath = fs.mkdtempSync(path.join(os.tmpdir(), "gdl-empty-"))
    runtimePaths.push(runtimePath)
    expect(killGameProcesses(runtimePath)).toEqual([])
  })
})

describe("windowsPathSpellings", () => {
  it("adds the expanded spelling of a short (8.3) runtime path", () => {
    const short = "C:\\Users\\PLAYER~1\\AppData\\Local\\Temp\\gdl-e2e-x"
    const long = "C:\\Users\\player-one\\AppData\\Local\\Temp\\gdl-e2e-x"
    expect(windowsPathSpellings(short, () => long)).toEqual([short, long])
  })

  it("keeps one spelling when nothing expands", () => {
    const long = "C:\\Users\\player-one\\gdl-e2e-x"
    expect(windowsPathSpellings(long, (p) => p)).toEqual([long])
  })

  it("falls back to the given spelling when the path cannot be resolved", () => {
    const gone = "C:\\Users\\PLAYER~1\\gone"
    expect(
      windowsPathSpellings(gone, () => {
        throw new Error("ENOENT")
      })
    ).toEqual([gone])
  })
})

describe("describeLauncherProcesses", () => {
  it.skipIf(process.platform === "win32")(
    "lists a running process with its start and CPU time",
    async () => {
      const marker = `gdl-describe-${process.pid}-${Date.now()}`
      const child = spawnChildMarkedWith(marker)
      await new Promise((resolve) => child.once("spawn", resolve))

      const listing = describeLauncherProcesses([marker])

      expect(listing).toContain(String(child.pid))
      expect(listing).toContain(marker)
      expect(listing.split("\n")[0]).toMatch(/PID.*STARTED.*TIME/)
    },
    PROCESS_TEST_TIMEOUT
  )

  it.skipIf(process.platform === "win32")(
    "says none when nothing runs the binaries",
    () => {
      expect(
        describeLauncherProcesses([`gdl-nothing-runs-this-${Date.now()}`])
      ).toBe("(none)")
    }
  )
})
