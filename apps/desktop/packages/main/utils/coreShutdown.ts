import type { ChildProcess } from "node:child_process"
import { randomUUID } from "node:crypto"
import { canonicalRuntimePath } from "./runtimeRelocation"

/** Timeout/error/signal failure are failure, never process-exit proof. */
export function terminateAndObserveCoreExit(
  proc: ChildProcess,
  timeoutMs = 5000
): Promise<void> {
  if (proc.exitCode !== null || proc.signalCode !== null)
    return Promise.resolve()
  return new Promise((resolve, reject) => {
    let settled = false
    const finish = (error?: Error) => {
      if (settled) return
      settled = true
      clearTimeout(timer)
      proc.off("exit", exited)
      proc.off("error", failed)
      if (error) reject(error)
      else resolve()
    }
    const exited = () => finish()
    const failed = (error: Error) => finish(error)
    const timer = setTimeout(
      () =>
        finish(
          new Error("Core exit was not confirmed; runtime source retained")
        ),
      timeoutMs
    )
    proc.once("exit", exited)
    proc.once("error", failed)
    if (proc.exitCode !== null || proc.signalCode !== null) {
      finish()
      return
    }
    try {
      if (!proc.kill()) finish(new Error("Failed to signal core shutdown"))
    } catch (error) {
      finish(error as Error)
    }
  })
}
/**
 * Ends the core even when it does not take part: SIGTERM first, which a core
 * that can still settle answers by stopping as usual, then SIGKILL after
 * `graceMs`. Needed because on macOS and Linux the core's own SIGTERM
 * handler goes through the same handshake that just refused or did not
 * finish. Resolves only once the exit is observed; rejects if even SIGKILL
 * is not seen to end it within `killWaitMs`. On Windows both signals end the
 * process at once.
 *
 * Local servers that are still saving already received 'stop' and are not
 * tied to the core (kill_on_drop(false)), so they finish on their own; the
 * next start shows any still running as left by an earlier session.
 */
export function forceTerminateCore(
  proc: ChildProcess,
  graceMs = 3000,
  killWaitMs = 5000
): Promise<void> {
  const exited = () => proc.exitCode !== null || proc.signalCode !== null
  if (exited()) {
    requests.delete(proc)
    return Promise.resolve()
  }
  return new Promise((resolve, reject) => {
    let settled = false
    const timers: Partial<
      Record<"kill" | "giveUp", ReturnType<typeof setTimeout>>
    > = {}
    const finish = (error?: Error) => {
      if (settled) return
      settled = true
      clearTimeout(timers.kill)
      clearTimeout(timers.giveUp)
      proc.off("exit", onExit)
      proc.off("error", ignore)
      if (error) reject(error)
      else {
        requests.delete(proc)
        resolve()
      }
    }
    const onExit = () => finish()
    // A failed signal is reported as 'error'; SIGKILL or the timeout decides.
    const ignore = () => {}
    const signal = (name: NodeJS.Signals) => {
      try {
        proc.kill(name)
      } catch (error) {
        console.warn(`Could not send ${name} to the core`, error)
      }
    }
    proc.once("exit", onExit)
    proc.on("error", ignore)
    if (exited()) return finish()
    signal("SIGTERM")
    timers.kill = setTimeout(() => {
      if (exited()) return finish()
      signal("SIGKILL")
      timers.giveUp = setTimeout(
        () =>
          exited()
            ? finish()
            : finish(new Error("Core exit was not confirmed after SIGKILL")),
        killWaitMs
      )
    }, graceMs)
  })
}
export interface RelocationCore {
  port: number
  apiToken: string
  process: ChildProcess | null
}
/**
 * What keeps the core from moving its runtime folder, or a quit, restart or
 * update from finishing (core RuntimeBlocker).
 */
export interface RuntimeBlocker {
  kind:
    | "game"
    | "server"
    | "install"
    | "deletion"
    | "task"
    | "unconfirmed"
    | "previousGame"
    | "previousProcess"
    | "unreadableRecord"
    | "startupCheckPending"
    | "startupCheckFailed"
    | "shutdown"
    | "relocation"
  name: string
  /**
   * For a process from an earlier session: its process ID, which is how
   * Task Manager and Activity Monitor tell processes apart.
   */
  pid?: number
  /**
   * The program that holds that process ID now (`javaw.exe`, `java`), when
   * the core could read it. It tells the user whether the PID is Java
   * before they end anything.
   */
  program?: string
}
/**
 * The core is still serving normally: it refused the request (and lists why)
 * or released it again before committing. Nothing was frozen, cancelled or
 * stopped, so the caller may let the user dismiss and carry on.
 */
export class ShutdownRefusedError extends Error {
  readonly blockers: RuntimeBlocker[]
  constructor(message: string, blockers: RuntimeBlocker[] = []) {
    super(message)
    this.name = "ShutdownRefusedError"
    this.blockers = blockers
  }
}
/** The core answered with an HTTP error, as opposed to a lost answer. */
class CoreAnswerError extends Error {
  readonly status: number
  readonly body: string
  constructor(status: number, body: string) {
    super(`Core answered ${status}: ${body}`)
    this.status = status
    this.body = body
  }
  /** A 4xx is the handler's own refusal; a 5xx says nothing about the lease. */
  get definite() {
    return this.status >= 400 && this.status < 500
  }
  json(): Record<string, unknown> {
    try {
      const parsed = JSON.parse(this.body)
      return parsed && typeof parsed === "object" ? parsed : {}
    } catch {
      return {}
    }
  }
}
export type ShutdownPurpose = "quit" | "update"
/** The core's view of a quit, restart or update request. */
export interface ShutdownStatus {
  requestId: string
  purpose: ShutdownPurpose
  phase: "draining" | "ready" | "committed" | "unresolved"
  blockers: RuntimeBlocker[]
  serversStopped: boolean
}
/**
 * Why an attempt to stop the core for a quit, restart or update ended
 * without an observed exit:
 * - `busy`: the core refused or no longer holds the request. Nothing is
 *   frozen for it, so a new attempt starts over.
 * - `unresolved`: the core stopped its work but something did not finish;
 *   it holds the request, and admission stays frozen until it is retried,
 *   forced or cancelled.
 * - `slow`: still finishing when the launcher stopped waiting.
 * - `unreachable`: the core did not answer.
 * - `exit`: the core agreed to stop, but its exit was not seen.
 */
export type ShutdownBlockedReason =
  | "busy"
  | "unresolved"
  | "slow"
  | "unreachable"
  | "exit"
export class ShutdownBlockedError extends Error {
  readonly reason: ShutdownBlockedReason
  readonly blockers: RuntimeBlocker[]
  readonly serversStopped: boolean
  constructor(
    message: string,
    reason: ShutdownBlockedReason,
    blockers: RuntimeBlocker[] = [],
    serversStopped = false
  ) {
    super(message)
    this.name = "ShutdownBlockedError"
    this.reason = reason
    this.blockers = blockers
    this.serversStopped = serversStopped
  }
}
/** Blockers from a status, also from a core that still sent plain text. */
function blockersOf(status: { blockers?: unknown }): RuntimeBlocker[] {
  if (!Array.isArray(status?.blockers)) return []
  return status.blockers.map((blocker) =>
    typeof blocker === "string"
      ? { kind: "task", name: blocker }
      : (blocker as RuntimeBlocker)
  )
}
const describe = (blockers: RuntimeBlocker[]) =>
  blockers.map((blocker) => `${blocker.kind}: ${blocker.name}`).join("; ")
/**
 * The quit, restart or update request each core process holds for this
 * launcher, with the last phase seen. Kept while the core may still hold it,
 * so a retry or a lost commit ACK joins it; dropped once the core says it
 * holds nothing for it, after a cancel, and once the core has exited.
 */
const requests = new WeakMap<
  ChildProcess,
  { requestId: string; purpose: ShutdownPurpose; phase?: string }
>()
const sleep = (ms: number, signal?: AbortSignal) =>
  new Promise<void>((resolve) => {
    if (signal?.aborted) return resolve()
    const timer = setTimeout(done, ms)
    function done() {
      clearTimeout(timer)
      signal?.removeEventListener("abort", done)
      resolve()
    }
    signal?.addEventListener("abort", done, { once: true })
  })

/**
 * Asks the core to stop for a quit, restart or update, waits for it to
 * settle, commits and observes the exit. Repeated attempts join the same
 * authenticated request, even after lost ACKs, and retry it once it is
 * unresolved. A request the core holds as unresolved under the other
 * purpose is replaced by a new one; one still draining is joined under its
 * own purpose, as the core treats both alike.
 *
 * Throws ShutdownBlockedError when the exit was not reached; `signal` stops
 * waiting early (as `slow`), also for an answer still on its way, without
 * giving up the request: the core keeps it until it is cancelled.
 */
export async function stopCoreForShutdown(
  core: RelocationCore,
  purpose: ShutdownPurpose,
  transport: typeof fetch = fetch,
  timeoutMs = 5000,
  options: {
    signal?: AbortSignal
    onStatus?: (status: ShutdownStatus) => void
    deadlineMs?: number
  } = {}
): Promise<void> {
  const proc = core.process
  if (!proc) throw new Error("Shutdown requires a launcher-owned core")
  const exited = () => proc.exitCode !== null || proc.signalCode !== null
  if (exited()) {
    requests.delete(proc)
    return
  }
  let held = requests.get(proc)
  if (held && held.purpose !== purpose && held.phase === "unresolved")
    held = undefined
  if (!held) {
    held = { requestId: randomUUID(), purpose }
    requests.set(proc, held)
  }
  let owned = held
  const forget = () => {
    if (requests.get(proc) === owned) requests.delete(proc)
  }
  /**
   * Starts over under a new id. The core retired the old one: an earlier
   * Keep GDLauncher open reached it although its answer did not.
   */
  const renew = () => {
    forget()
    owned = { requestId: randomUUID(), purpose }
    requests.set(proc, owned)
  }
  // A user who chose to stop now or keep GDLauncher open does not wait for
  // a core that is slow to answer.
  const requestSignal = () =>
    options.signal
      ? AbortSignal.any([AbortSignal.timeout(timeoutMs), options.signal])
      : AbortSignal.timeout(timeoutMs)
  async function request(endpoint: string) {
    const response = await transport(
      `http://127.0.0.1:${core.port}/runtime/${endpoint}`,
      {
        method: "POST",
        headers: {
          "content-type": "application/json",
          "x-api-token": core.apiToken
        },
        body: JSON.stringify({
          requestId: owned.requestId,
          purpose: owned.purpose
        }),
        signal: requestSignal()
      }
    )
    if (!response.ok)
      throw new CoreAnswerError(response.status, await response.text())
    const data = await response.json()
    if (data.requestId !== owned.requestId)
      throw new Error("Core shutdown acknowledgment mismatch")
    return data
  }
  const definite = (error: unknown) =>
    error instanceof CoreAnswerError && error.definite
  const retired = (error: unknown) =>
    definite(error) && (error as CoreAnswerError).json().cancelled === true
  /** The core holds nothing under this id, so nothing is frozen for it. */
  const released = (error: CoreAnswerError) => {
    forget()
    const body = error.json()
    return new ShutdownBlockedError(
      `Core shutdown refused: ${typeof body.error === "string" ? body.error : error.body}`,
      "busy",
      blockersOf(body)
    )
  }
  const seen = (status: ShutdownStatus) => {
    status = { ...status, blockers: blockersOf(status) }
    owned.phase = status.phase
    options.onStatus?.(status)
    return status
  }

  // Preparing is idempotent. Retry a lost preparation ACK with the same ID;
  // a local boolean can never authorize cancelling a server drain.
  async function prepare() {
    try {
      return await request("prepare-shutdown")
    } catch (error) {
      if (definite(error)) throw error
      return await request("prepare-shutdown")
    }
  }
  let preparation
  try {
    try {
      preparation = await prepare()
    } catch (error) {
      if (!retired(error)) throw error
      renew()
      preparation = await prepare()
    }
  } catch (error) {
    if (definite(error)) throw released(error as CoreAnswerError)
    throw new ShutdownBlockedError(
      `Core did not answer the shutdown request: ${String(error)}`,
      "unreachable"
    )
  }
  // Longer than the core's own worst case, so a core that is still working
  // is never cut off: about 42 s for a server to save, be killed and log its
  // last lines, then up to 5 s to hand running games over. The user sees
  // what it waits for long before this, and can stop or keep it at any time.
  const deadline = Date.now() + (options.deadlineMs ?? 60000)
  let status = seen(preparation.status)
  while (
    status.phase === "draining" &&
    Date.now() < deadline &&
    !options.signal?.aborted &&
    !exited()
  ) {
    await sleep(100, options.signal)
    if (options.signal?.aborted || exited()) break
    try {
      status = seen((await request("shutdown-status")).status)
    } catch (error) {
      if (definite(error)) throw released(error as CoreAnswerError)
      // A lost answer: ask again until the deadline, unless the core is
      // gone (the loop ends on that).
    }
  }
  // The core exited while it drained: it crashed, or was ended outside the
  // launcher. Nothing is left to stop, as when it had exited before the
  // attempt, so the prompt does not wait out the deadline naming work that
  // ended with it.
  if (exited()) {
    forget()
    return
  }
  if (status.purpose !== owned.purpose || status.requestId !== owned.requestId)
    throw new Error("Shutdown status ownership mismatch")
  if (status.phase === "draining")
    throw new ShutdownBlockedError(
      `Shutdown is still finishing: ${describe(status.blockers)}`,
      "slow",
      status.blockers,
      status.serversStopped
    )
  if (status.phase === "unresolved")
    throw new ShutdownBlockedError(
      `Shutdown remains unresolved: ${describe(status.blockers)}`,
      "unresolved",
      status.blockers,
      status.serversStopped
    )
  if (status.phase !== "committed") {
    try {
      await request("shutdown")
    } catch (error) {
      let recovered
      try {
        recovered = (await request("shutdown-status")).status
      } catch {
        // Unknown: the next attempt asks again under the same id.
      }
      if (
        recovered?.phase !== "committed" ||
        recovered.requestId !== owned.requestId ||
        recovered.purpose !== owned.purpose
      )
        throw new ShutdownBlockedError(
          `Core shutdown commit failed: ${String(error)}`,
          "unreachable",
          status.blockers,
          status.serversStopped
        )
    }
  }
  owned.phase = "committed"
  try {
    await terminateAndObserveCoreExit(proc, timeoutMs)
  } catch (error) {
    throw new ShutdownBlockedError(String(error), "exit")
  }
  forget()
}
/**
 * Hands a quit, restart or update request back after the user chose to keep
 * GDLauncher open: the core opens admission again. Returns `released` when
 * the core holds nothing for this launcher any more, or `finishing` when it
 * can no longer go back (already committed, or its games already handed
 * over) and the caller should let the stop finish. Throws when the core did
 * not answer; the request is then kept for the next attempt.
 */
export async function cancelCoreShutdown(
  core: RelocationCore,
  transport: typeof fetch = fetch,
  timeoutMs = 5000
): Promise<"released" | "finishing"> {
  const proc = core.process
  const held = proc && requests.get(proc)
  if (!proc || !held) return "released"
  const response = await transport(
    `http://127.0.0.1:${core.port}/runtime/cancel-shutdown`,
    {
      method: "POST",
      headers: {
        "content-type": "application/json",
        "x-api-token": core.apiToken
      },
      body: JSON.stringify({ requestId: held.requestId }),
      signal: AbortSignal.timeout(timeoutMs)
    }
  )
  if (!response.ok) {
    const error = new CoreAnswerError(response.status, await response.text())
    if (!error.definite) throw error
    const body = error.json()
    if (body.committed === true || body.finishing === true) return "finishing"
  }
  if (requests.get(proc) === held) requests.delete(proc)
  return "released"
}
/**
 * One relocation lease per core process, kept until the core exits or the
 * lease is released. A retry after a lost ACK or a slow exit re-joins it.
 * Separate from `requests`: a move never claims quit, restart or update.
 */
const relocations = new WeakMap<ChildProcess, string>()

/**
 * Whether the core may still hold a move lease for this launcher: an earlier
 * attempt's lease that was not confirmed released or ended. Without one, an
 * attempt that ends before stopCoreForRelocation changed nothing in the core.
 */
export function relocationLeaseHeld(proc: ChildProcess | null): boolean {
  return !!proc && relocations.has(proc)
}

const relocationNotLive =
  "Relocation requires a live launcher-owned core; restart normally and retry"

/**
 * Asks the core, before the slow scan of the data folder, whether anything
 * would refuse the move right now. It takes no lease and changes nothing.
 * Throws ShutdownRefusedError with the blockers when something would; any
 * other failure is left to stopCoreForRelocation, which asks again anyway.
 */
export async function checkRelocation(
  core: RelocationCore,
  transport: typeof fetch = fetch,
  timeoutMs = 5000
): Promise<void> {
  const proc = core.process
  // Nothing to freeze: the move simply cannot start from this launcher.
  if (!proc) throw new ShutdownRefusedError(relocationNotLive)
  if (proc.exitCode !== null || proc.signalCode !== null)
    throw new Error(relocationNotLive)
  let blockers: RuntimeBlocker[] = []
  try {
    const response = await transport(
      `http://127.0.0.1:${core.port}/runtime/relocation-blockers`,
      {
        method: "POST",
        headers: {
          "content-type": "application/json",
          "x-api-token": core.apiToken
        },
        // An earlier attempt's own lease is not something in the way.
        body: JSON.stringify({ requestId: relocations.get(proc) ?? "" }),
        signal: AbortSignal.timeout(timeoutMs)
      }
    )
    if (!response.ok) return
    const data = await response.json()
    if (Array.isArray(data?.blockers)) blockers = data.blockers
  } catch (error) {
    console.warn("Relocation blocker check failed; the lease decides", error)
    return
  }
  if (blockers.length)
    throw new ShutdownRefusedError(
      "Core relocation refused: runtime is busy",
      blockers
    )
}

/**
 * Refuse-first move handshake: /prepare-relocation checks every game, server,
 * download and unverified process before it changes anything, /shutdown
 * commits the lease, and only then is the core stopped and its exit observed.
 * A 409 refusal throws ShutdownRefusedError with the core's blockers.
 */
export async function stopCoreForRelocation(
  core: RelocationCore,
  source: string,
  transport: typeof fetch = fetch,
  timeoutMs = 5000
): Promise<void> {
  const proc = core.process
  if (!proc) throw new Error(relocationNotLive)
  if (proc.exitCode !== null || proc.signalCode !== null)
    throw new Error(relocationNotLive)
  let requestId = relocations.get(proc)
  if (!requestId) {
    requestId = randomUUID()
    relocations.set(proc, requestId)
  }
  const forget = () => {
    if (relocations.get(proc) === requestId) relocations.delete(proc)
  }
  /** Starts over under a new id; the old one holds nothing. */
  const renew = () => {
    forget()
    requestId = randomUUID()
    relocations.set(proc, requestId)
  }
  async function request(endpoint: string) {
    const response = await transport(
      `http://127.0.0.1:${core.port}/runtime/${endpoint}`,
      {
        method: "POST",
        headers: {
          "content-type": "application/json",
          "x-api-token": core.apiToken
        },
        body: JSON.stringify({ requestId }),
        signal: AbortSignal.timeout(timeoutMs)
      }
    )
    if (!response.ok)
      throw new CoreAnswerError(response.status, await response.text())
    const data = await response.json()
    if (data.requestId !== requestId)
      throw new Error("Core relocation acknowledgment mismatch")
    return data
  }
  /** A refusal from prepare-relocation means this id holds no lease. */
  const refused = (error: CoreAnswerError) => {
    forget()
    const body = error.json()
    return new ShutdownRefusedError(
      `Core relocation refused: ${typeof body.error === "string" ? body.error : error.body}`,
      Array.isArray(body.blockers) ? (body.blockers as RuntimeBlocker[]) : []
    )
  }
  /**
   * Hands the lease back after a failure before commit. Resolves only when
   * the commit is known to have landed; otherwise throws: a release as
   * ShutdownRefusedError, an unknown outcome as the original error so the
   * caller keeps the core and a retry re-joins the same lease.
   */
  async function release(cause: unknown): Promise<void> {
    try {
      await request("cancel-relocation")
    } catch (error) {
      if (!(error instanceof CoreAnswerError) || !error.definite) throw cause
      if (error.json().committed === true) return
      // Otherwise no lease with this id exists, so nothing is frozen.
    }
    forget()
    throw new ShutdownRefusedError(
      cause instanceof Error ? cause.message : String(cause)
    )
  }

  /** A definite refusal that only says this id was cancelled earlier. */
  const cancelledId = (error: unknown) =>
    error instanceof CoreAnswerError &&
    error.definite &&
    error.json().cancelled === true
  async function lease() {
    try {
      return await request("prepare-relocation")
    } catch (error) {
      if (error instanceof CoreAnswerError && error.definite) throw error
      // A lost ACK: the same id re-joins whatever lease the core granted.
      try {
        return await request("prepare-relocation")
      } catch (retryError) {
        if (retryError instanceof CoreAnswerError && retryError.definite)
          throw retryError
        await release(retryError)
        throw retryError
      }
    }
  }

  let preparation
  try {
    try {
      preparation = await lease()
    } catch (error) {
      // An earlier attempt's release reached the core but its answer did
      // not: the core holds nothing for that id, so ask under a new one.
      if (!cancelledId(error)) throw error
      renew()
      preparation = await lease()
    }
  } catch (error) {
    if (error instanceof CoreAnswerError && error.definite) throw refused(error)
    throw error
  }
  try {
    if (
      (await canonicalRuntimePath(preparation.runtimePath)) !==
      (await canonicalRuntimePath(source))
    )
      throw new Error("Core runtime differs from the selected source")
  } catch (error) {
    await release(error)
    throw error
  }
  try {
    await request("shutdown")
  } catch (error) {
    // Refused or unanswered: only the lease itself can say whether it landed.
    await release(error)
  }
  await terminateAndObserveCoreExit(proc, timeoutMs)
  forget()
}
