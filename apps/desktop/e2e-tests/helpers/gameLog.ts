/**
 * Reads the per-launch game log the launcher captures for a running instance.
 *
 * When `launch_account.is_some()`, `run/mod.rs` writes Minecraft's own output
 * to `<instance_root>/logs/<YYYY-MM-DD_HH-MM-SS>.log`, wrapped one message
 * per log4j event (`format_message_as_log4j_event`). It also prefixes its own
 * preamble — the resolved Java, the library list, the client jar — through
 * the same channel, so the file mixes launcher-authored and game-authored
 * lines. Nothing here distinguishes them; callers assert on content.
 *
 * The pure parsing and matching live here rather than in the spec so they are
 * unit-testable without a running game, following `installVerify.ts`'s split.
 * `waitForLogQuiescence` is the one exception — a real Playwright polling
 * loop, not a pure function — kept here anyway because it is built directly
 * on `newestLogFile`/`logSize` above and is shared by every spec that drives
 * a real launch (`gameLaunch.spec.ts`, `modpackLifecycle.spec.ts`).
 */

import fs from "node:fs"
import path from "node:path"
import { expect, test } from "@playwright/test"

/** Matches one log4j event's message payload, including multi-line ones —
 *  the launcher pretty-prints Rust structs across a dozen lines inside a
 *  single event. */
const MESSAGE_RE = /<log4j:Message><!\[CDATA\[([\s\S]*?)\]\]><\/log4j:Message>/g

/**
 * Corroborating startup-log markers; they do not witness a menu or response.
 *
 * Minecraft logs no explicit title-screen event, so every entry here is a
 * proxy rather than the thing itself — and each one's exact wording has
 * changed at some point in Minecraft's history (the atlas line's `WxHxD`
 * format gained its depth field; older builds phrased the sound engine
 * differently). That is precisely why the assertion built on this is a
 * quorum rather than a conjunction: any single line going stale on a
 * Minecraft release must not turn the suite red.
 *
 * Deliberately loose, and deliberately loader-agnostic: no Forge or NeoForge
 * banner appears here, so the same pool works for Fabric and Quilt launches
 * without a per-loader table. Also deliberately excluded is the Realms
 * authorization failure — the most title-screen-specific line in a real log,
 * but an artifact of the e2e mock account that would stop matching the
 * moment anyone ran this with a real one.
 *
 * When a version drifts, update this list — it is the single place to do so.
 */
export const LAUNCH_MARKERS: RegExp[] = [
  /Setting user:/,
  /LWJGL/,
  /Sound engine started/i,
  /Reloading ResourceManager/,
  /atlas/i
]

/** How many of `LAUNCH_MARKERS` must appear. Below the pool size on purpose:
 *  see that constant's doc comment. */
export const LAUNCH_MARKER_QUORUM = 3

/**
 * Signatures that mean the JVM itself died, as opposed to the game reporting
 * something it recovered from.
 *
 * Kept narrow on purpose. A real launch under the e2e mock account always
 * logs `Failed to verify authentication`, `Failed to retrieve profile key
 * pair` and a Realms authorization failure, because the mock token is not
 * one any session server accepts. Matching on "error" or "failed" would make
 * this permanently red while proving nothing.
 */
const FATAL_SIGNATURES: RegExp[] = [
  /Exception in thread "main"/,
  /A fatal error has been detected by the Java Runtime/,
  /Could not (?:find or )?load main class/
]

/**
 * Driver messages that mean the machine gives the game no OpenGL context, so
 * no launch here can get past window creation and the startup this file
 * observes cannot happen. Not a launcher failure: the game was found, started
 * and got as far as asking for a window.
 *
 * Seen on GitHub's windows-2022 runners, which have no GPU and no OpenGL
 * driver of their own, before CI installed Mesa there (`all_os.yml`): Forge's
 * early window logged "Failed to find a valid GLFW profile" with "GLFW error:
 * [0x10006]WGL: The driver does not appear to support OpenGL" for every
 * version from 4.6 down to 3.2 (develop run 32960118348).
 * GitHub's macOS runners fail the same way with "NSGL: Failed to find a
 * suitable pixel format" (run 41466362930).
 * Add a signature only from a real log; a guess would skip real failures.
 */
const GRAPHICS_UNAVAILABLE_SIGNATURES: RegExp[] = [
  /WGL: The driver does not appear to support OpenGL/,
  /NSGL: Failed to find a suitable pixel format/,
  // The JVM's fatal-error banner, which it prints raw rather than as a log4j
  // event, naming the Windows OpenGL driver as the frame that crashed. A
  // Fabric pack crashed this way inside `GL.createCapabilities` on
  // windows-2022 without logging a WGL message first (run 41774657124).
  /^# C {2}\[opengl32\.dll\+0x[0-9a-f]+\]/
]

/**
 * Set by CI (`all_os.yml`) on the GitHub runners that have no OpenGL driver
 * (macOS), where no game can open its window. Unset everywhere else.
 */
const NO_OPENGL_DECLARED = process.env.GDL_E2E_NO_OPENGL === "1"

/**
 * Set by CI (`all_os.yml`) on the runners where games get a software OpenGL
 * driver (Linux, and Windows through Mesa). There a log naming no driver means
 * that setup broke, so the test fails instead of skipping: otherwise a broken
 * install would quietly turn back into skips.
 */
const OPENGL_REQUIRED = process.env.GDL_E2E_REQUIRE_OPENGL === "1"

/** Logged as Minecraft starts its window system, just before the window. */
const WINDOW_STARTING = "Backend library: LWJGL version"

/** Logged only once the window is open: the later `LAUNCH_MARKERS`. */
const WINDOW_OPENED: RegExp[] = [
  /Sound engine started/i,
  /Reloading ResourceManager/,
  /atlas/i
]

/** The first log line naming a missing OpenGL driver, or `undefined`. */
export function findGraphicsUnavailable(
  messages: string[]
): string | undefined {
  for (const message of messages) {
    const line = message
      .split("\n")
      .find((l) => GRAPHICS_UNAVAILABLE_SIGNATURES.some((s) => s.test(l)))
    if (line !== undefined) return line.trim()
  }
  return undefined
}

/**
 * For a game that died or went quiet: its window-system line, when it
 * stopped after starting its window system and before its window opened, on
 * a machine declared to have no OpenGL driver. Otherwise `undefined`.
 *
 * Some packs stop at that step without naming the driver. On macos-15 the
 * system killed one (SIGKILL, no crash report) right after this line (run
 * 41774657124), and in the next run the same pack hung there instead (run
 * 41799780827). Only trusted where `noOpenGl` is declared: on a machine with
 * a driver, a game stopping there is a real failure.
 */
export function findStopOpeningWindow(
  messages: string[],
  noOpenGl: boolean
): string | undefined {
  if (!noOpenGl) return undefined
  if (messages.some((m) => WINDOW_OPENED.some((marker) => marker.test(m))))
    return undefined
  return messages.find((m) => m.startsWith(WINDOW_STARTING))
}

/**
 * What the game log `text` of a game that died or went quiet means for the
 * running test, given what CI declares about the machine's OpenGL:
 * - `skip` when the log names no OpenGL driver the game can use (see
 *   `GRAPHICS_UNAVAILABLE_SIGNATURES`), or shows the game stopped opening its
 *   window where `noOpenGl` is declared (`findStopOpeningWindow`);
 * - `fail` instead for a log naming no driver where `openGlRequired` is
 *   declared: that machine was set up with one, so the setup broke;
 * - `undefined` for a log without either: the caller's own assertions decide.
 *
 * The reason carries the log line that decided it.
 */
export function judgeGraphicsUnavailable(
  text: string,
  { noOpenGl, openGlRequired }: { noOpenGl: boolean; openGlRequired: boolean }
): { outcome: "skip" | "fail"; reason: string } | undefined {
  const messages = parseLogMessages(text)
  // The raw text last: it also holds what the JVM printed outside any event.
  const line = findGraphicsUnavailable([...messages, text])
  if (line !== undefined && openGlRequired) {
    return {
      outcome: "fail",
      reason:
        "the game found no OpenGL driver it can use, on a machine CI set up " +
        "to give games one (GDL_E2E_REQUIRE_OPENGL): check that setup in " +
        `all_os.yml. Game log: ${line}`
    }
  }
  if (line !== undefined) {
    return {
      outcome: "skip",
      reason:
        "this machine has no OpenGL driver the game can use, so its startup " +
        `cannot be observed here. Game log: ${line}`
    }
  }
  const windowLine = findStopOpeningWindow(messages, noOpenGl)
  if (windowLine !== undefined) {
    return {
      outcome: "skip",
      reason:
        "this machine has no OpenGL driver (GDL_E2E_NO_OPENGL) and the game " +
        `stopped opening its window, after: ${windowLine}`
    }
  }
  return undefined
}

/**
 * Skips the running test, for a game that died or went quiet, when the
 * newest game log under `logsDir` shows the machine has no OpenGL driver, or
 * that the game stopped opening its window on a machine declared to have
 * none. Where CI declares games have OpenGL (`GDL_E2E_REQUIRE_OPENGL`), a log
 * naming no driver fails the test instead. See `judgeGraphicsUnavailable`.
 * The reason, with the log line, lands in the report. A log without either
 * changes nothing: the caller's own assertions still decide.
 */
export function skipIfGraphicsUnavailable(logsDir: string): void {
  const logFile = newestLogFile(logsDir)
  const text =
    logFile && fs.existsSync(logFile) ? fs.readFileSync(logFile, "utf8") : ""
  const verdict = judgeGraphicsUnavailable(text, {
    noOpenGl: NO_OPENGL_DECLARED,
    openGlRequired: OPENGL_REQUIRED
  })
  if (verdict?.outcome === "fail") throw new Error(verdict.reason)
  test.skip(verdict?.outcome === "skip", verdict?.reason)
}

/** Every message payload in `raw`, in file order. */
export function parseLogMessages(raw: string): string[] {
  return [...raw.matchAll(MESSAGE_RE)].map((match) => match[1])
}

/**
 * How many of `markers` matched at least one message.
 *
 * Distinct markers, not total matches: a real startup stitches eleven
 * atlases, and counting matches would let one chatty subsystem reach quorum
 * by itself while every other subsystem stayed silent.
 */
export function countMatchedMarkers(
  messages: string[],
  markers: RegExp[]
): number {
  return markers.filter((marker) => messages.some((m) => marker.test(m))).length
}

/** The first fatal signature found, or `undefined` if the log holds none. */
export function findFatalSignature(messages: string[]): string | undefined {
  for (const message of messages) {
    if (FATAL_SIGNATURES.some((signature) => signature.test(message))) {
      return message
    }
  }
  return undefined
}

/** `<instance_root>/logs` — where `run/mod.rs` puts per-launch game logs. */
export function instanceLogsDir(
  runtimePath: string,
  shortpath: string
): string {
  return path.join(runtimePath, "instances", shortpath, "logs")
}

/**
 * The newest log file in `logsDir`, or `undefined` before the launcher has
 * created one. Names are `YYYY-MM-DD_HH-MM-SS`, so lexical order is
 * chronological order.
 */
export function newestLogFile(logsDir: string): string | undefined {
  if (!fs.existsSync(logsDir)) return undefined
  const entries = fs.readdirSync(logsDir).sort()
  if (entries.length === 0) return undefined
  return path.join(logsDir, entries[entries.length - 1])
}

/** Size in bytes, or 0 when the file does not exist yet. */
export function logSize(logFile: string | undefined): number {
  if (!logFile || !fs.existsSync(logFile)) return 0
  return fs.statSync(logFile).size
}

/** Every message currently in `logFile`. */
export function readLogMessages(logFile: string | undefined): string[] {
  if (!logFile || !fs.existsSync(logFile)) return []
  return parseLogMessages(fs.readFileSync(logFile, "utf8"))
}

/**
 * How long the client is given to produce its first byte of log output after
 * Play is clicked, inside `waitForLogQuiescence` below. Generous: a cold
 * instance re-resolves Java and assets first. The two callers
 * (`gameLaunch.spec.ts`, `modpackLifecycle.spec.ts`) independently arrived at
 * this same 180s figure for their own, separate GAME_LAUNCHED poll — this is
 * the stricter (i.e. no looser) of the two values either ever used for the
 * first-log-output wait specifically, kept as the single figure now that both
 * share one implementation.
 */
const LOG_FIRST_OUTPUT_TIMEOUT = 180_000

/** How long to wait for the game log to stop growing once it has started. */
const LOG_QUIESCENCE_TIMEOUT = 240_000

/** How long the log must hold steady for the quiet-output observation. Long
 *  enough to clear the gaps between startup phases — texture stitching
 *  pauses for seconds at a time under software GL — without waiting out the
 *  whole test budget. */
const LOG_QUIESCENCE_HOLD_MS = 20_000

/** Poll interval for log growth. */
const LOG_QUIESCENCE_POLL_MS = 2_000

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

/**
 * Waits for the newest game log under `logsDir` to grow and then stop
 * growing during a live-process observation. This is startup-output evidence;
 * it does not observe menu rendering or input responsiveness.
 * `isAlive` is polled throughout so a crash mid-load is reported as a crash
 * rather than as a slow, eventually-satisfied quiescence; both current
 * callers pass a predicate over their own tracked `GAME_CLOSED` count
 * compared against its pre-launch baseline.
 *
 * Skips the test (`skipIfGraphicsUnavailable`) when the client exited or
 * went quiet and its log shows the machine has no OpenGL driver, or that it
 * stopped opening its window on a machine CI declares has none. Where CI
 * declares games have OpenGL, a log naming no driver fails it instead.
 *
 * Growth is checked before stillness: a log that never grows at all means
 * the JVM produced nothing, a different failure from one that started and
 * stopped, worth distinguishing in the message.
 *
 * A single shared implementation: both `gameLaunch.spec.ts` and
 * `modpackLifecycle.spec.ts` call this rather than each keeping their own
 * copy. Silence and liveness alone do not establish responsiveness or menu rendering.
 */
export async function waitForLogQuiescence(
  logsDir: string,
  isAlive: () => boolean
): Promise<void> {
  await expect
    .poll(() => logSize(newestLogFile(logsDir)), {
      timeout: LOG_FIRST_OUTPUT_TIMEOUT,
      message:
        `no game log appeared under ${logsDir} after launch — the client ` +
        "produced no output at all"
    })
    .toBeGreaterThan(0)

  const deadline = Date.now() + LOG_QUIESCENCE_TIMEOUT
  let lastSize = -1
  let steadySince = Date.now()

  while (Date.now() < deadline) {
    const size = logSize(newestLogFile(logsDir))
    if (size !== lastSize) {
      lastSize = size
      steadySince = Date.now()
    } else if (Date.now() - steadySince >= LOG_QUIESCENCE_HOLD_MS) {
      break
    }

    // Checked every poll rather than once at the end: a client that died is
    // not going to start logging again, so continuing to wait for
    // quiescence would just burn the timeout before reporting the real
    // cause. A client that died for lack of an OpenGL driver skips instead,
    // or fails naming the driver where CI declares games have OpenGL.
    if (!isAlive()) skipIfGraphicsUnavailable(logsDir)
    expect(
      isAlive(),
      "the game exited during the startup-log observation"
    ).toBe(true)

    await sleep(LOG_QUIESCENCE_POLL_MS)
  }

  // A client that went quiet because it has no window to open is not one
  // whose startup can be judged by the callers' marker quorum.
  skipIfGraphicsUnavailable(logsDir)

  expect(
    Date.now() - steadySince,
    `the game log under ${logsDir} never stopped growing within ` +
      `${LOG_QUIESCENCE_TIMEOUT}ms — startup output did not become quiet`
  ).toBeGreaterThanOrEqual(LOG_QUIESCENCE_HOLD_MS)
}
