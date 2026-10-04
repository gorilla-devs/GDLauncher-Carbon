import type { FEServerState } from "@gd/core_module/bindings"

/**
 * What the main action of a server left behind by an earlier session of the
 * launcher does.
 *
 * - `stop`: its JVM still runs and can be asked to save and stop (macOS and
 *   Linux send it SIGTERM, which runs Minecraft's shutdown hook).
 * - `forceStop`: its JVM still runs but can only be ended at once (Windows),
 *   losing what changed since its last autosave, so the user confirms first.
 * - `clearRecord`: a record says it may still run but the core cannot verify
 *   it; the user confirms no server from that folder runs, then clears it.
 */
export type RecoveredServerAction = "stop" | "forceStop" | "clearRecord"

export function recoveredServerAction(
  state: FEServerState | undefined
): RecoveredServerAction | undefined {
  if (state?.status === "recoveredRunning") {
    return state.graceful_stop ? "stop" : "forceStop"
  }
  if (state?.status === "recoveredUnverified") return "clearRecord"
  return undefined
}

/** The PID the record names, when it could be read. */
export function recoveredServerPid(
  state: FEServerState | undefined
): number | null {
  if (
    state?.status === "recoveredRunning" ||
    state?.status === "recoveredUnverified"
  ) {
    return state.process_id
  }
  return null
}

/**
 * What `SERVER_RECOVERED_PROCESS` carries: a server's record from an earlier
 * session refused a start, delete or change (`RecoveredServerError`).
 */
export interface RecoveredServerRefusal {
  kind: "running" | "unverified" | "unreadable"
  pid: number | null
  graceful_stop: boolean
}

const refusalKeys = {
  running: "instances:_trn_server_recovered_refusal_running",
  unverified: "instances:_trn_server_recovered_refusal_unverified",
  unreadable: "instances:_trn_server_recovered_refusal_unreadable"
} as const

/**
 * A running server that cannot save and stop (Windows): its page offers
 * Force stop, not Stop, so the text also names the in-game /stop.
 */
const forceStopRefusalKey =
  "instances:_trn_server_recovered_refusal_running_force" as const

/**
 * The translated text for that refusal, which sends the user to the server
 * page instead of naming a button in English. `undefined` for a kind this
 * build does not know, which then shows the core's own text.
 */
export function recoveredServerRefusalKey(
  data: unknown
):
  | (typeof refusalKeys)[keyof typeof refusalKeys]
  | typeof forceStopRefusalKey
  | undefined {
  const refusal = data as Partial<RecoveredServerRefusal> | null | undefined
  const kind = refusal?.kind
  if (kind === "running" && refusal?.graceful_stop === false) {
    return forceStopRefusalKey
  }
  return typeof kind === "string" && Object.hasOwn(refusalKeys, kind)
    ? refusalKeys[kind]
    : undefined
}
