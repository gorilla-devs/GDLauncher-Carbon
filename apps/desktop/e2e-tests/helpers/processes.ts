/**
 * Finding and sweeping up processes this suite's harnesses spawned.
 *
 * Extracted from `dbRecovery.spec.ts`, which needed it first to clean up
 * relaunched app/core siblings, and now shared with `stopHarness`: once a
 * running game outlives the launcher (see `main.rs`'s termination handler),
 * teardown is the only thing standing between a failed test and a Minecraft
 * JVM that runs until the machine is rebooted.
 *
 * Everything here is best-effort: a failure reports "found nothing" rather
 * than throwing, so a caller sweeping leaked processes reads that as "leave
 * it alone", never as "assume it's mine and kill it".
 */

import { execSync } from "node:child_process"
import nodeFs from "node:fs"
import path from "node:path"

/**
 * The env var `launchApp` sets on every spawned app/core process to point it
 * at its own isolated runtime path (`fixtures/electronApp.ts`'s `env`
 * block). Inherited by everything those processes spawn in turn — including
 * the game JVM — which is what makes it usable for attribution below.
 */
const RUNTIME_PATH_ENV_VAR = "GDL_RUNTIME_PATH"

/**
 * Pids whose command line contains `needle`. Empty on no match.
 *
 * Case-insensitive on Windows, where the needle travels by environment so a
 * path containing quotes cannot change what runs.
 */
export function pidsMatching(needle: string): number[] {
  if (process.platform === "win32") {
    try {
      return execSync(
        "powershell -NoProfile -NonInteractive -Command " +
          '"Get-CimInstance Win32_Process | Where-Object { $_.CommandLine -and ' +
          "$_.CommandLine.ToLower().Contains($env:GDL_PID_NEEDLE.ToLower()) } | " +
          'ForEach-Object { $_.ProcessId }"',
        {
          encoding: "utf8",
          env: { ...process.env, GDL_PID_NEEDLE: needle }
        }
      )
        .split("\n")
        .map((s) => Number(s.trim()))
        .filter((n) => Number.isInteger(n) && n > 0)
    } catch {
      return []
    }
  }

  try {
    return execSync(`pgrep -f ${JSON.stringify(needle)}`, { encoding: "utf8" })
      .split("\n")
      .map((s) => Number(s.trim()))
      .filter((n) => Number.isInteger(n) && n > 0)
  } catch {
    // pgrep exits 1 with no output when nothing matches — not an error here.
    return []
  }
}

/**
 * Best-effort check that `pid` was launched with `GDL_RUNTIME_PATH` set to
 * `runtimePath`. For the app and core this is the only attribution available,
 * because they receive the path by environment rather than argv.
 *
 * `ps e` (BSD syntax, native on both Linux's procps and macOS) appends the
 * process environment to its output. Substring search rather than parsing
 * into KEY=VALUE pairs: `ps` output has no unambiguous delimiter between
 * adjacent env vars once values can contain spaces, and an exact
 * `GDL_RUNTIME_PATH=<path>` match is already specific enough that a false
 * positive would need another var whose value contains that same text.
 *
 * Any failure — the pid raced away, `ps` behaved unexpectedly, unsupported
 * platform — reports "cannot confirm" rather than throwing. A caller
 * sweeping up leaked processes must read that as "leave it alone", never as
 * "assume it's mine and kill it".
 */
export function pidRuntimePathMatches(
  pid: number,
  runtimePath: string
): boolean {
  try {
    const output = execSync(`ps e -ww -p ${pid}`, { encoding: "utf8" })
    return output.includes(`${RUNTIME_PATH_ENV_VAR}=${runtimePath}`)
  } catch {
    return false
  }
}

/**
 * `process.kill(pid, 0)` is Node's documented cross-platform existence
 * probe: signal `0` sends nothing, it just asks the OS whether `pid` is
 * still addressable. `ESRCH` is the only error meaning "gone" — anything
 * else (`EPERM`, a pid that exists but isn't ours to signal) means alive.
 */
export function isPidAlive(pid: number): boolean {
  try {
    process.kill(pid, 0)
    return true
  } catch (error) {
    return (error as NodeJS.ErrnoException).code !== "ESRCH"
  }
}

/**
 * Kills any game JVM belonging to `runtimePath`, returning the pids killed.
 *
 * Matched on `<runtimePath>/managed_javas`, the launcher's own managed JRE
 * directory, which appears in the JVM's command line and is unique to this
 * harness — never on a bare `java`, which would kill a developer's unrelated
 * JVM. Attribution is then confirmed a second way, through the inherited
 * `GDL_RUNTIME_PATH`, before anything is signalled: a pid that cannot be
 * confirmed is left alone.
 */
/** Kills `pid` and its descendants; Electron's children outlive a bare kill. */
export function killProcessTree(pid: number): void {
  try {
    if (process.platform === "win32") {
      execSync(`taskkill /PID ${pid} /T /F`, { stdio: "ignore" })
    } else {
      process.kill(pid, "SIGKILL")
    }
  } catch {
    // Already gone, or never ours.
  }
}

/** `PID_FILE_NAME` in `managers/instance/mod.rs`. */
const PID_FILE_NAME = ".gdl_instance.pid"

/** The full command line `pid` is running, or null when it cannot be read. */
function commandLineOf(pid: number): string | null {
  try {
    if (process.platform === "win32") {
      // Filtered by WMI rather than enumerated and filtered in PowerShell:
      // `Get-CimInstance Win32_Process | Where-Object ...` walks every process
      // on the machine, which cost enough on a loaded CI runner to time the
      // unit tests out. ProcessId is an integer, so the filter needs no
      // quoting or escaping.
      return (
        execSync(
          "powershell -NoProfile -NonInteractive -Command " +
            "\"Get-CimInstance Win32_Process -Filter ('ProcessId = ' + " +
            '$env:GDL_PID) | ForEach-Object { $_.CommandLine }"',
          { encoding: "utf8", env: { ...process.env, GDL_PID: String(pid) } }
        ).trim() || null
      )
    }

    return (
      execSync(`ps -p ${pid} -o args=`, { encoding: "utf8" }).trim() || null
    )
  } catch {
    return null
  }
}

/**
 * Whether `pid` is still the process a pidfile under `runtimePath` recorded.
 *
 * The launcher verifies a recorded pid by start time (`orphan_pid.rs`) because
 * a user's runtime path is stable, so a recycled pid could legitimately name
 * it. Here every run gets its own temp runtime path, so the path appearing in
 * the process's own command line is already proof of identity, and a recycled
 * pid cannot fake it.
 */
export function pidBelongsToRun(pid: number, runtimePath: string): boolean {
  const command = commandLineOf(pid)
  if (!command) {
    return false
  }

  return process.platform === "win32"
    ? command.toLowerCase().includes(runtimePath.toLowerCase())
    : command.includes(runtimePath)
}

/** Pids recorded by every instance pidfile under `runtimePath`. */
function recordedGamePids(runtimePath: string): number[] {
  const instancesDir = path.join(runtimePath, "instances")

  let entries: string[]
  try {
    entries = nodeFs.readdirSync(instancesDir)
  } catch {
    return []
  }

  return entries
    .map((entry) => path.join(instancesDir, entry, PID_FILE_NAME))
    .flatMap((pidFile) => {
      try {
        const pid = Number(
          nodeFs.readFileSync(pidFile, "utf8").split("\n")[0].trim()
        )
        return Number.isInteger(pid) && pid > 0 ? [pid] : []
      } catch {
        return []
      }
    })
}

export function killGameProcesses(runtimePath: string): number[] {
  const killed: number[] = []

  for (const pid of recordedGamePids(runtimePath)) {
    if (!isPidAlive(pid)) continue
    if (!pidBelongsToRun(pid, runtimePath)) continue

    try {
      process.kill(pid, "SIGKILL")
      killed.push(pid)
    } catch {
      // Raced away between the liveness probe and here; nothing to do.
    }
  }

  return killed
}
