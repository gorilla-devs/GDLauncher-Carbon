/**
 * Finding and sweeping up processes this suite's harnesses spawned.
 *
 * Extracted from `dbRecovery.spec.ts`, which needed it first to clean up
 * relaunched app/core siblings, and now shared with `stopHarness`: once a
 * running game outlives the launcher (see `main.rs`'s termination handler),
 * teardown is the only thing standing between a failed test and a Minecraft
 * JVM that runs until the machine is rebooted.
 *
 * Everything here is best-effort and Unix-only. `pgrep`/`ps`/`kill` have no
 * equivalent wired up for Windows, so on win32 these degrade to "found
 * nothing" rather than throwing — a miss leaves one idle process for the
 * remainder of an already-ephemeral CI VM, which is a better trade than a
 * second, less-tested cleanup path.
 */

import { execSync } from "node:child_process"

/**
 * The env var `launchApp` sets on every spawned app/core process to point it
 * at its own isolated runtime path (`fixtures/electronApp.ts`'s `env`
 * block). Inherited by everything those processes spawn in turn — including
 * the game JVM — which is what makes it usable for attribution below.
 */
const RUNTIME_PATH_ENV_VAR = "GDL_RUNTIME_PATH"

/** Pids whose command line contains `needle`. Empty on win32 and on no match. */
export function pidsMatching(needle: string): number[] {
  if (process.platform === "win32") return []

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
 * `runtimePath` — the only thing tying a matching process back to *this*
 * harness rather than another worker's, since the runtime path travels by
 * env rather than argv.
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
export function killGameProcesses(runtimePath: string): number[] {
  const killed: number[] = []

  for (const pid of pidsMatching(`${runtimePath}/managed_javas`)) {
    if (!isPidAlive(pid)) continue
    if (!pidRuntimePathMatches(pid, runtimePath)) continue

    try {
      process.kill(pid, "SIGKILL")
      killed.push(pid)
    } catch {
      // Raced away between the liveness probe and here; nothing to do.
    }
  }

  return killed
}
