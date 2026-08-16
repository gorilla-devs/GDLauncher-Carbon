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
import { expect } from "@playwright/test"

/** Matches one log4j event's message payload, including multi-line ones —
 *  the launcher pretty-prints Rust structs across a dozen lines inside a
 *  single event. */
const MESSAGE_RE = /<log4j:Message><!\[CDATA\[([\s\S]*?)\]\]><\/log4j:Message>/g

/**
 * Proxies for "the client finished starting and is sitting at the main menu".
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

/** How long the log must hold steady to count as "finished loading". Long
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
 * growing — the operational proxy this suite uses for "reached a stable main
 * menu" wherever it drives a real launch (see `gameLaunch.spec.ts`'s module
 * doc comment for why no log wording is trusted as the primary signal).
 * `isAlive` is polled throughout so a crash mid-load is reported as a crash
 * rather than as a slow, eventually-satisfied quiescence; both current
 * callers pass a predicate over their own tracked `GAME_CLOSED` count
 * compared against its pre-launch baseline.
 *
 * Growth is checked before stillness: a log that never grows at all means
 * the JVM produced nothing, a different failure from one that started and
 * stopped, worth distinguishing in the message.
 *
 * Previously two independent copies — one inlined in `gameLaunch.spec.ts`'s
 * test body, one a local function in `modpackLifecycle.spec.ts` explicitly
 * documented as a copy of the former. Centralized here; both files now call
 * this instead.
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
    // cause.
    expect(
      isAlive(),
      "the game exited while still loading — it crashed rather than " +
        "reaching the main menu"
    ).toBe(true)

    await sleep(LOG_QUIESCENCE_POLL_MS)
  }

  expect(
    Date.now() - steadySince,
    `the game log under ${logsDir} never stopped growing within ` +
      `${LOG_QUIESCENCE_TIMEOUT}ms — the client never finished loading`
  ).toBeGreaterThanOrEqual(LOG_QUIESCENCE_HOLD_MS)
}
