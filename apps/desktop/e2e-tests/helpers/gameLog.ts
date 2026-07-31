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
 */

import fs from "node:fs"
import path from "node:path"

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
