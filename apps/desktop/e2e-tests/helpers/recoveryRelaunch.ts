import { execFileSync } from "node:child_process"
import fs from "node:fs"
import {
  readE2EMainStart,
  type E2EMainStart
} from "../../packages/main/utils/e2eBootstrap"
import {
  observeMainProcess,
  type MainProcessIdentity,
  type ProcessIdentityIO
} from "../../packages/main/utils/e2eMainIdentity"
import { isPidAlive, killProcessTree } from "./processes"

export interface OutgoingMainProcess {
  pid?: number
  exitCode: number | null
  signalCode: NodeJS.Signals | null
  once: (event: "exit", listener: () => void) => unknown
  off: (event: "exit", listener: () => void) => unknown
}

export interface RelaunchAttempt {
  before: E2EMainStart
  outgoingExited: () => boolean
  dispose: () => void
}

/** Subscribe before clicking Restart; an already-closed original is not a
 * valid outgoing main for this operation. The marker must name the exact PID
 * of the main process the test is connected to, `mainPid`, read from inside
 * it (`app.evaluate(() => process.pid)`). That is `outgoing.pid` itself
 * except on Windows, where Playwright starts Electron through a shell
 * (`shell: true` in its electron launcher), so `outgoing` is the shell. The
 * shell waits for Electron, so its exit still marks the outgoing main's exit.
 */
export function captureRelaunchAttempt(
  outgoing: OutgoingMainProcess,
  expected: { userDataPath: string; runtimePath: string; runId: string },
  mainPid: number | undefined = outgoing.pid
): RelaunchAttempt {
  const before = readE2EMainStart(expected.userDataPath)
  if (
    !before ||
    mainPid === undefined ||
    before.process.pid !== mainPid ||
    before.runId !== expected.runId ||
    before.runtimePath !== expected.runtimePath ||
    outgoing.exitCode !== null ||
    outgoing.signalCode !== null
  )
    throw new Error("Cannot attribute outgoing E2E main")
  let exited = false
  const onExit = () => {
    exited = true
  }
  outgoing.once("exit", onExit)
  return {
    before,
    outgoingExited: () => exited,
    dispose: () => {
      outgoing.off("exit", onExit)
    }
  }
}

export interface RelaunchWaitDeps {
  readMarker: (path: string) => E2EMainStart | null
  observe: (pid: number) => MainProcessIdentity | null
  now: () => number
  sleep: (ms: number) => Promise<void>
}
const nativeDeps: RelaunchWaitDeps = {
  readMarker: readE2EMainStart,
  observe: observeMainProcess,
  now: Date.now,
  sleep: (ms) => new Promise((resolve) => setTimeout(resolve, ms))
}

/** Fresh main generation in the same owned userData/runtime/run, actual
 * outgoing exit, and matching executable/start identity must all agree.
 * No healthy core is required for a DB_CORRUPT relaunch. Outgoing PID reuse,
 * observed start/executable mismatches, unknown state and helpers are refused.
 * macOS ps start identity has one-second precision: same-second same-binary
 * candidate PID reuse is not distinguishable by that native observation.
 */
export async function waitForRelaunchMain(
  attempt: RelaunchAttempt,
  timeoutMs = 15_000,
  deps: RelaunchWaitDeps = nativeDeps
): Promise<MainProcessIdentity> {
  const deadline = deps.now() + timeoutMs
  const before = attempt.before
  try {
    for (;;) {
      const marker = deps.readMarker(before.userDataPath)
      if (
        attempt.outgoingExited() &&
        marker &&
        marker.generation !== before.generation &&
        marker.runId === before.runId &&
        marker.runtimePath === before.runtimePath &&
        marker.userDataPath === before.userDataPath &&
        marker.process.pid !== before.process.pid &&
        marker.process.executable === before.process.executable
      ) {
        const current = deps.observe(marker.process.pid)
        if (
          current?.role === "main" &&
          current.pid === marker.process.pid &&
          current.executable === marker.process.executable &&
          current.startIdentity === marker.process.startIdentity
        )
          return current
      }
      if (deps.now() >= deadline)
        throw new Error(
          "Restart did not produce an attributable fresh main after outgoing main exit"
        )
      await deps.sleep(50)
    }
  } finally {
    attempt.dispose()
  }
}

export interface StopRelaunchDeps {
  readMarker: (path: string) => E2EMainStart | null
  observe: (pid: number) => MainProcessIdentity | null
  /** Ends `pid` and everything under it; returns the pids it ended. */
  killTree: (pid: number) => number[]
  isAlive: (pid: number) => boolean
  now: () => number
  sleep: (ms: number) => Promise<void>
}

export interface StoppedRelaunch {
  main: number
  killed: number[]
  /** Pids still running when the wait ran out. */
  survivors: number[]
}

/** taskkill names each process it ends, one line each: "SUCCESS: The process
 *  with PID 1234 (child process of PID 5678) has been terminated." */
export function parseTaskkillEnded(output: string): number[] {
  return output.split(/\r?\n/).flatMap((line) => {
    const match = /^SUCCESS:.*?\bPID (\d+)/.exec(line.trim())
    return match ? [Number(match[1])] : []
  })
}

function killTreeNative(pid: number): number[] {
  if (process.platform !== "win32") {
    killProcessTree(pid)
    return [pid]
  }
  let output: string
  try {
    output = execFileSync("taskkill", ["/PID", String(pid), "/T", "/F"], {
      encoding: "utf8",
      windowsHide: true
    })
  } catch (error) {
    // Non-zero when part of the tree was already gone; what it did end is
    // still listed on stdout.
    const { stdout } = error as { stdout?: string | Buffer }
    output = stdout?.toString() ?? ""
  }
  return parseTaskkillEnded(output)
}

/** `observeMainProcess`'s own probes give up after 10 s, which a very busy
 *  Windows runner can exceed. Teardown waits longer rather than leave the
 *  main running. */
const patientIO: ProcessIdentityIO = {
  readFile: (file) => fs.readFileSync(file, "utf8"),
  realpath: (file) => fs.realpathSync(file),
  execFile: (file, args) =>
    execFileSync(file, args, {
      encoding: "utf8",
      windowsHide: true,
      timeout: 30_000
    }).trim()
}

const nativeStopDeps: StopRelaunchDeps = {
  readMarker: readE2EMainStart,
  observe: (pid) => observeMainProcess(pid, process.platform, patientIO),
  killTree: killTreeNative,
  isAlive: isPidAlive,
  now: Date.now,
  sleep: (ms) => new Promise((resolve) => setTimeout(resolve, ms))
}

/** Stops the main a Restart started, with everything under it: its core and
 * Chromium's helpers. `app.relaunch()` starts it outside Playwright, so
 * nothing else ends it. Left running, it keeps the harness's runtime folder
 * open (its userData lives there), so Windows cannot delete the folder, and
 * after a database reset it stays on the login screen, whose looping
 * background video keeps a GPU-less runner busy for the rest of the job.
 *
 * Only a main the marker attributes to `before`'s run, runtime and userData,
 * from a later generation and another pid, whose live identity still matches
 * the marker, is stopped. Otherwise nothing is touched and null is returned.
 */
export async function stopRelaunchedMain(
  before: E2EMainStart,
  timeoutMs = 15_000,
  deps: StopRelaunchDeps = nativeStopDeps
): Promise<StoppedRelaunch | null> {
  const marker = deps.readMarker(before.userDataPath)
  if (
    !marker ||
    marker.generation === before.generation ||
    marker.runId !== before.runId ||
    marker.runtimePath !== before.runtimePath ||
    marker.userDataPath !== before.userDataPath ||
    marker.process.pid === before.process.pid ||
    marker.process.executable !== before.process.executable
  )
    return null
  const main = marker.process.pid
  const live = deps.observe(main)
  if (
    live?.role !== "main" ||
    live.pid !== main ||
    live.executable !== marker.process.executable ||
    live.startIdentity !== marker.process.startIdentity
  )
    return null
  const killed = [...new Set([main, ...deps.killTree(main)])]
  const deadline = deps.now() + timeoutMs
  let survivors = killed.filter((pid) => deps.isAlive(pid))
  while (survivors.length > 0 && deps.now() < deadline) {
    await deps.sleep(100)
    survivors = survivors.filter((pid) => deps.isAlive(pid))
  }
  return { main, killed, survivors }
}
