import type {
  RuntimeBlocker,
  ShutdownBlockedReason,
  ShutdownStatus
} from "./coreShutdown"
import { ShutdownBlockedError } from "./coreShutdown"

/** What stopping the core is for. Each one says so in its prompt. */
export type ShutdownAction = "quit" | "update" | "restart" | "reset" | "restore"
/**
 * The user's answer to a quit, restart or update that has not finished:
 * - `wait`: keep waiting (while it still runs) or try again (once it ended);
 * - `force`: end the core now and carry on with the quit, update or restart;
 * - `keep`: give up on it and keep using GDLauncher.
 */
export type ShutdownChoice = "wait" | "force" | "keep"
/** Everything a prompt shows. Sent to the renderer as is. */
export interface ShutdownView {
  action: ShutdownAction
  /**
   * `waiting`: still finishing. `blocked`: the attempt ended without
   * stopping. `stopping`: the user chose to stop now and the core is being
   * ended; nothing to choose.
   */
  stage: "waiting" | "blocked" | "stopping"
  reason: ShutdownBlockedReason | null
  /** What is still running or did not finish, in the core's order. */
  blockers: RuntimeBlocker[]
  /**
   * Offers Wait (`waiting`: a quit only, whose window then goes away while
   * it finishes; anything else keeps waiting with the prompt on screen) or
   * Try again (`blocked`). Neither while something waiting cannot clear is
   * in the way.
   */
  canWait: boolean
  /**
   * The user chose Keep GDLauncher open, but the core was already handing
   * running games over and could no longer go back: it carries on, and
   * Keep GDLauncher open is no longer offered.
   */
  finishing: boolean
  /** Local servers were asked to stop and stay stopped if the user goes back. */
  serversStopped: boolean
}
export type ShutdownOutcome = "stopped" | "forced" | "kept"
/**
 * The stop cannot be done from this launcher at all (a database reset with
 * a development core it does not own). Nothing to wait for or force, so it
 * is not asked about: the flow fails with it.
 */
export class ShutdownNotPossibleError extends Error {
  constructor(message: string) {
    super(message)
    this.name = "ShutdownNotPossibleError"
  }
}
export interface ShutdownFlowIO {
  /**
   * One attempt: asks the core to stop and resolves once its exit is
   * observed. Throws (ShutdownBlockedError when the core said why) when it
   * did not get there. `signal` ends the wait early; the core keeps the
   * request, so the next attempt joins it.
   */
  stop(
    signal: AbortSignal,
    onStatus: (status: ShutdownStatus) => void
  ): Promise<void>
  /**
   * Gives the request back so the core accepts work again. `released`: done;
   * `finishing`: the core is already on its way out and cannot go back.
   */
  cancel(): Promise<"released" | "finishing">
  /** Ends the core (SIGTERM, then SIGKILL) and resolves on its exit. */
  force(): Promise<void>
  /**
   * Shows `view` in the launcher window and resolves with the choice, or
   * with null once `signal` aborts it or the window went away unanswered.
   */
  prompt(
    view: ShutdownView,
    signal: AbortSignal
  ): Promise<ShutdownChoice | null>
  /**
   * Shows `view` in place of what the prompt asked with `signal` says, as
   * what the stop waits for changes. A prompt that cannot change (the
   * native dialog) may keep what it says.
   */
  update?(view: ShutdownView, signal: AbortSignal): void
  /** Quit only: the window goes away while the quit finishes unseen. */
  hide?(): void
  log?(message: string, error?: unknown): void
}
interface Attempt {
  done: Promise<void>
  result?: { error?: unknown }
  abort(): void
}
const never = new AbortController().signal
const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms))
/** Before asking again after a prompt went away unanswered. */
const pause = () => sleep(250)
/**
 * Between an attempt that ended only because something is still finishing
 * and the next one. Each attempt lasts the core's 15 s operation budget;
 * this only keeps one that fails at once from spinning.
 */
const RETRY_PAUSE_MS = 1000
/**
 * An attempt that ends sooner than this did not wait on anything: work that
 * is still finishing holds an attempt for the core's 15 s operation budget,
 * a server's save for up to 30 s.
 */
const QUICK_ATTEMPT_MS = 2000
/**
 * Quick attempts in a row after which the stop is no longer tried again
 * without asking: whatever fails that fast is not finishing, and waiting
 * on it would retry unseen for good.
 */
const QUICK_ATTEMPTS_TRIED = 3
/**
 * Work whose end the core could not confirm, also a server drain that
 * failed: trying again only repeats it, so waiting or trying again cannot
 * help.
 */
const waitingCannotHelp = new Set<string>(["unconfirmed"])
const cannotSettleByWaiting = (blockers: RuntimeBlocker[]) =>
  blockers.some((blocker) => waitingCannotHelp.has(blocker.kind))
/**
 * An attempt that ended only because something is still finishing: the
 * core holds the request, and trying it again gets there once that is done.
 */
function waitable(error: unknown) {
  return (
    error instanceof ShutdownBlockedError &&
    (error.reason === "unresolved" || error.reason === "slow") &&
    !cannotSettleByWaiting(error.blockers)
  )
}
/** What a prompt shows of `view`: one row per kind, so names do not count. */
function shownAs(view: ShutdownView) {
  return JSON.stringify({
    ...view,
    blockers: [...new Set(view.blockers.map((blocker) => blocker.kind))]
  })
}

/**
 * Runs a quit, restart or update to its end with the user in charge of how
 * long it may take. The first `promptAfterMs` pass unseen; a stop that is
 * still running then shows the `waiting` prompt, and one that ended without
 * stopping the `blocked` prompt, each with a force action and Keep
 * GDLauncher open, plus Wait or Try again where waiting can help.
 *
 * Waiting means waiting until it is done: while the `waiting` prompt is on
 * screen, and after the user chose Wait or Try again, an attempt that ends
 * only because something is still finishing (a long deletion, copy or
 * server save) is tried again under the same request, without asking. The
 * `blocked` prompt comes back for what waiting cannot fix, and for attempts
 * that keep failing at once. The `waiting` prompt follows what the stop
 * waits for while it is on screen.
 *
 * A refused or cancelled attempt never leaves the core frozen: Keep
 * GDLauncher open gives its request back.
 */
export class ShutdownFlow {
  private wake?: () => void
  /** See ShutdownView.finishing. */
  private finishing = false
  private latest: Pick<ShutdownStatus, "blockers" | "serversStopped"> = {
    blockers: [],
    serversStopped: false
  }
  /** Attempts in a row that ended within QUICK_ATTEMPT_MS. */
  private quickAttempts = 0
  /** Tells the prompt on screen that what it would say may have changed. */
  private changed?: () => void
  constructor(
    readonly action: ShutdownAction,
    private readonly io: ShutdownFlowIO,
    private readonly promptAfterMs = 3000
  ) {}
  /** Shows the prompt now if the stop is still running unseen. */
  reveal() {
    this.wake?.()
  }
  /**
   * The pause before trying the same request again unseen. reveal() cuts it
   * short; returns whether it did, so the prompt is shown at once.
   */
  private async pauseBeforeRetry(): Promise<boolean> {
    let revealed = false
    await new Promise<void>((resolve) => {
      const timer = setTimeout(resolve, RETRY_PAUSE_MS)
      this.wake = () => {
        revealed = true
        clearTimeout(timer)
        resolve()
      }
    })
    this.wake = undefined
    return revealed
  }
  private start(): Attempt {
    // What an earlier attempt named is not what this one waits for.
    this.latest = { blockers: [], serversStopped: this.latest.serversStopped }
    const controller = new AbortController()
    const attempt: Attempt = {
      abort: () => controller.abort(),
      done: Promise.resolve()
    }
    const started = Date.now()
    attempt.done = this.io
      .stop(controller.signal, (status) => {
        this.latest = status
        this.changed?.()
      })
      .then(
        () => {
          attempt.result = {}
        },
        (error) => {
          attempt.result = { error }
          this.quickAttempts =
            Date.now() - started < QUICK_ATTEMPT_MS ? this.quickAttempts + 1 : 0
          if (error instanceof ShutdownBlockedError && error.blockers.length)
            this.latest = {
              blockers: error.blockers,
              serversStopped: error.serversStopped
            }
          // An unresolved request can be given back again.
          if (
            error instanceof ShutdownBlockedError &&
            error.reason === "unresolved"
          )
            this.finishing = false
          this.changed?.()
        }
      )
    return attempt
  }
  /**
   * Whether the stop is tried again without asking: the attempt ended only
   * because something is still finishing, and attempts are not failing at
   * once again and again.
   */
  private retryable(error: unknown) {
    return waitable(error) && this.quickAttempts < QUICK_ATTEMPTS_TRIED
  }
  /**
   * Resolves once the attempt ended, `promptAt` (a Date.now() time; never
   * when undefined) came or reveal() was called.
   */
  private async quiet(attempt: Attempt, promptAt: number | undefined) {
    let timer: ReturnType<typeof setTimeout> | undefined
    const woken = new Promise<void>((resolve) => {
      this.wake = resolve
      if (promptAt !== undefined)
        timer = setTimeout(resolve, Math.max(0, promptAt - Date.now()))
    })
    await Promise.race([attempt.done, woken])
    clearTimeout(timer)
    this.wake = undefined
  }
  private view(stage: ShutdownView["stage"], error?: unknown): ShutdownView {
    const reason =
      stage !== "blocked"
        ? null
        : error instanceof ShutdownBlockedError
          ? error.reason
          : "unreachable"
    // Once stopping, nothing is waited for any more.
    const blockers = stage === "stopping" ? [] : this.latest.blockers
    return {
      action: this.action,
      stage,
      reason,
      blockers,
      canWait:
        stage !== "stopping" &&
        // The waiting prompt is the wait itself unless the window can go.
        (stage === "blocked" || this.io.hide !== undefined) &&
        !cannotSettleByWaiting(blockers),
      finishing: this.finishing,
      serversStopped: this.latest.serversStopped
    }
  }
  /**
   * Keep GDLauncher open: true once the core gave the request back, or did
   * not answer twice (it may still hold the request, which the next attempt
   * joins; the user still gets the launcher back). False when the core was
   * already finishing: the stop goes on, and the next prompt says so.
   */
  private async keep(): Promise<boolean> {
    for (let tries = 1; ; tries++) {
      try {
        if ((await this.io.cancel()) === "released") return true
        this.finishing = true
        return false
      } catch (error) {
        this.io.log?.("Could not hand the shutdown request back", error)
        if (tries === 2) return true
      }
    }
  }
  /**
   * The user chose to stop now. A quit's window goes away at once, as it
   * does for any quit. Anything else keeps its window, which says it is
   * stopping until the core's exit is seen: on macOS and Linux that takes
   * a few seconds, as the core's own SIGTERM handler is refused while the
   * request is pending and SIGKILL follows.
   */
  private async forceNow(): Promise<"forced"> {
    if (this.io.hide) {
      this.io.hide()
      await this.io.force()
      return "forced"
    }
    const stopping = new AbortController()
    const shown = this.io
      .prompt(this.view("stopping"), stopping.signal)
      .catch(() => null)
    try {
      await this.io.force()
    } finally {
      stopping.abort()
      await shown
    }
    return "forced"
  }
  async run(): Promise<ShutdownOutcome> {
    let attempt = this.start()
    // When the waiting prompt is due if the stop is still running then:
    // undefined while it runs unseen until reveal().
    let promptAt: number | undefined = Date.now() + this.promptAfterMs
    // The user chose Wait or Try again.
    let waiting = false
    for (;;) {
      if (!attempt.result) await this.quiet(attempt, promptAt)
      const ended = attempt.result
      if (ended?.error instanceof ShutdownNotPossibleError) throw ended.error
      if (ended && !("error" in ended)) return "stopped"
      if (ended && waiting && this.retryable(ended.error)) {
        // Unseen for a quit, or before the prompt is due. The same
        // `promptAt` holds across retries, so retries that fail fast never
        // put the prompt off.
        if (await this.pauseBeforeRetry()) promptAt = Date.now()
        attempt = this.start()
        continue
      }
      if (!ended) {
        // Still running: say what it waits for, and keep saying it as that
        // changes. While on screen it keeps waiting, and closes by itself
        // once the stop finished or failed in a way waiting cannot fix.
        const prompt = new AbortController()
        const first = this.view("waiting")
        let shown = shownAs(first)
        const answer = this.io.prompt(first, prompt.signal)
        this.changed = () => {
          const view = this.view("waiting")
          if (shownAs(view) === shown) return
          shown = shownAs(view)
          this.io.update?.(view, prompt.signal)
        }
        let choice: ShutdownChoice | null | undefined
        try {
          for (;;) {
            choice = await Promise.race([
              answer,
              attempt.done.then(() => undefined)
            ])
            if (choice !== undefined || !this.retryable(attempt.result?.error))
              break
            await sleep(RETRY_PAUSE_MS)
            attempt = this.start()
          }
        } finally {
          this.changed = undefined
        }
        if (choice === undefined) {
          prompt.abort()
          await answer.catch(() => null)
          promptAt = Date.now()
          continue
        }
        if (choice === null) {
          // The window went away unanswered: ask again.
          await pause()
          promptAt = Date.now()
          continue
        }
        if (choice === "wait") {
          // Only a quit offers it: the window goes away until it is done.
          waiting = true
          this.quickAttempts = 0
          this.io.hide?.()
          promptAt = this.io.hide ? undefined : Date.now() + this.promptAfterMs
          continue
        }
        attempt.abort()
        if (choice === "force") return this.forceNow()
        await attempt.done
        // It finished just as the user answered.
        if (attempt.result && !("error" in attempt.result)) return "stopped"
        if (await this.keep()) return "kept"
        // Too late to go back: say so at once.
        attempt = this.start()
        promptAt = Date.now()
        continue
      }
      // Ended without stopping.
      const choice = await this.io.prompt(
        this.view("blocked", ended.error),
        never
      )
      if (choice === null) {
        await pause()
        continue
      }
      if (choice === "force") return this.forceNow()
      if (choice === "wait") {
        waiting = true
        this.quickAttempts = 0
        this.io.hide?.()
        attempt = this.start()
        // A quit tries again unseen: the user already saw what is in the
        // way, and the window comes back only for what waiting cannot fix.
        // Anything else says what it waits for again 3 s after the click,
        // however many attempts that spans.
        promptAt = this.io.hide ? undefined : Date.now() + this.promptAfterMs
        continue
      }
      if (await this.keep()) return "kept"
      attempt = this.start()
      promptAt = Date.now()
    }
  }
}
