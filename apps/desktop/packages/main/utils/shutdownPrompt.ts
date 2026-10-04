import type { ShutdownChoice, ShutdownView } from "./shutdownFlow"

export interface PromptWindow {
  isDestroyed(): boolean
  show(): void
  focus(): void
  webContents: { send(channel: string, ...args: unknown[]): void }
}
/** What the renderer receives on "shutdown-prompt": a view, or null to close it. */
export interface ShutdownPromptMessage {
  id: number
  view: ShutdownView | null
}
const choices = new Set<string>(["wait", "force", "keep"])

/**
 * Shows a stop that did not finish in the launcher window and collects the
 * user's choice. A window whose App has mounted the prompt shows it in its
 * own language; anything else (the loading or startup error screen, a
 * renderer that is reloading) gets a native dialog parented to the window.
 *
 * The App reports each prompt it has on screen ("shutdown-prompt-shown").
 * One it does not report within `shownWithinMs` (a renderer that hangs, or
 * a prompt it cannot show) is closed there and asked with the native dialog
 * instead, so the window's close button, which is held while a prompt is
 * open, never leaves the user without a visible way out.
 *
 * A prompt with nothing to choose (the core is being ended) is only shown
 * by an App: a dialog needs a button, and that prompt goes away by itself
 * within seconds.
 */
export class ShutdownPrompts<W extends PromptWindow> {
  /** Renderers whose App listens for the prompt. */
  private receivers = new WeakSet<object>()
  private pending = new Map<
    number,
    {
      contents: W["webContents"]
      signal: AbortSignal
      /** What it says now; what the native dialog asks if it is not seen. */
      view: ShutdownView
      finish(choice: ShutdownChoice | null): void
      shown(): void
    }
  >()
  private nextId = 0
  private nativeOpen = 0
  /** Prompts with nothing to choose that no App could show. */
  private unshown = 0
  constructor(
    private readonly native: (
      window: W,
      view: ShutdownView,
      signal: AbortSignal
    ) => Promise<ShutdownChoice | null>,
    private readonly shownWithinMs = 2000
  ) {}
  /** Whether a prompt is up, and the window's close button held for it. */
  get open() {
    return this.pending.size > 0 || this.nativeOpen > 0 || this.unshown > 0
  }
  /** Records whether `contents`' App can show the prompt. A document that
   *  goes away takes its unanswered prompt with it. */
  receiver(contents: object, ready: boolean) {
    if (ready) {
      this.receivers.add(contents)
      return
    }
    this.receivers.delete(contents)
    for (const [, prompt] of [...this.pending])
      if (prompt.contents === contents) prompt.finish(null)
  }
  /** The renderer has prompt `id` on screen. Anything else is ignored. */
  shown(contents: object, id: unknown) {
    const prompt = typeof id === "number" ? this.pending.get(id) : undefined
    if (prompt?.contents === contents) prompt.shown()
  }
  /** The renderer's answer to prompt `id`. Anything else is ignored. */
  answer(contents: object, id: unknown, choice: unknown) {
    const prompt = typeof id === "number" ? this.pending.get(id) : undefined
    if (prompt?.contents !== contents) return
    if (typeof choice !== "string" || !choices.has(choice)) return
    prompt.finish(choice as ShutdownChoice)
  }
  /**
   * Shows `view` in place of what the App's prompt asked with `signal`
   * says, under the same id. A native dialog cannot change: it keeps what
   * it says.
   */
  update(signal: AbortSignal, view: ShutdownView) {
    for (const [id, prompt] of this.pending) {
      if (prompt.signal !== signal) continue
      prompt.view = view
      try {
        prompt.contents.send("shutdown-prompt", { id, view })
      } catch {
        // The window is already gone, and its prompt with it.
      }
    }
  }
  /** Shows `window` with `view` and resolves with the choice, or with null
   *  once `signal` aborts or the window's App went away unanswered. */
  ask(
    window: W,
    view: ShutdownView,
    signal: AbortSignal
  ): Promise<ShutdownChoice | null> {
    if (signal.aborted || window.isDestroyed()) return Promise.resolve(null)
    window.show()
    window.focus()
    const contents = window.webContents
    const silent = shutdownPromptLines(view).buttons.length === 0
    if (!this.receivers.has(contents))
      return silent
        ? this.holdUnshown(signal)
        : this.askNatively(window, view, signal)
    const id = ++this.nextId
    return new Promise((resolve) => {
      const close = () => {
        try {
          contents.send("shutdown-prompt", { id, view: null })
        } catch {
          // The window is already gone, and its prompt with it.
        }
      }
      const finish = (choice: ShutdownChoice | null) => {
        if (!this.pending.delete(id)) return
        clearTimeout(unseen)
        signal.removeEventListener("abort", aborted)
        resolve(choice)
      }
      const aborted = () => {
        close()
        finish(null)
      }
      const unseen = setTimeout(() => {
        const prompt = this.pending.get(id)
        if (!prompt) return
        if (silent) return aborted()
        this.pending.delete(id)
        signal.removeEventListener("abort", aborted)
        close()
        resolve(this.askNatively(window, prompt.view, signal))
      }, this.shownWithinMs)
      this.pending.set(id, {
        contents,
        signal,
        view,
        finish,
        shown: () => clearTimeout(unseen)
      })
      signal.addEventListener("abort", aborted, { once: true })
      try {
        contents.send("shutdown-prompt", { id, view })
      } catch {
        finish(null)
      }
    })
  }
  private askNatively(window: W, view: ShutdownView, signal: AbortSignal) {
    if (signal.aborted || window.isDestroyed()) return Promise.resolve(null)
    this.nativeOpen++
    return this.native(window, view, signal).finally(() => {
      this.nativeOpen--
    })
  }
  /** Holds the window's close button until `signal` aborts. */
  private holdUnshown(signal: AbortSignal) {
    if (signal.aborted) return Promise.resolve(null)
    this.unshown++
    return new Promise<null>((resolve) =>
      signal.addEventListener(
        "abort",
        () => {
          this.unshown--
          resolve(null)
        },
        { once: true }
      )
    )
  }
}

/**
 * The prompt's English text, keyed like `app:_trn_shutdown_prompt.<key>` in
 * packages/i18n. The App sends the same keys in the user's language
 * ("shutdown-prompt-text"); these stand in until it has.
 */
export const SHUTDOWN_PROMPT_ENGLISH = {
  title_quit_waiting: "Closing GDLauncher…",
  title_quit_blocked: "GDLauncher can't close yet",
  title_update_waiting: "Getting ready to install the update…",
  title_update_blocked: "The update can't be installed yet",
  title_restart_waiting: "Restarting GDLauncher…",
  title_restart_blocked: "GDLauncher can't restart yet",
  body_waiting:
    "GDLauncher is finishing what it was doing and carries on as soon as that is done.",
  body_unresolved: "Something GDLauncher was doing did not finish in time.",
  body_busy:
    "GDLauncher's background service is busy with another request and did not stop.",
  body_unreachable: "GDLauncher's background service is not responding.",
  body_exit:
    "GDLauncher's background service agreed to stop but has not closed yet.",
  body_stopping: "Stopping GDLauncher's background service…",
  blocker_game: "A game is still being prepared",
  blocker_server: "A local server is still stopping and saving its world",
  blocker_install: "A download or install is still running",
  blocker_deletion: "An instance or server is still being deleted",
  blocker_task:
    "A launcher task, such as an import, export, copy or scan, is still running",
  blocker_unconfirmed:
    "A task or local server did not confirm that it finished, so GDLauncher can't tell when it will be done",
  deletion_warning:
    "Stopping now can leave an instance or server that is being deleted only partly deleted. Delete it again after GDLauncher starts.",
  force_note:
    "Stopping now ends GDLauncher's background service right away. Downloads and installs that did not finish can be retried later. Servers that are still stopping will finish in the background.",
  servers_stopped:
    "Your local servers were asked to stop. If you keep GDLauncher open, start them again from the Library.",
  work_cancelled:
    "Downloads, installs, game launches and other tasks that were running were asked to stop. If you keep GDLauncher open, start again any that did not finish.",
  too_late_to_keep:
    "GDLauncher can no longer stay open: it is already finishing and carries on in a moment.",
  wait_hint_quit:
    "Wait hides this window. GDLauncher closes by itself once it is done.",
  wait: "Wait",
  try_again: "Try again",
  force_quit: "Quit now",
  force_update: "Install now",
  force_restart: "Restart now",
  force_reset: "Reset now",
  force_restore: "Restore now",
  keep_open: "Keep GDLauncher open"
} as const
export type ShutdownPromptKey = keyof typeof SHUTDOWN_PROMPT_ENGLISH
export type ShutdownPromptText = Partial<Record<ShutdownPromptKey, string>>
/** Keeps only known keys with text, so a renderer cannot inject anything else. */
export function shutdownPromptText(value: unknown): ShutdownPromptText {
  const text: ShutdownPromptText = {}
  if (!value || typeof value !== "object") return text
  for (const key of Object.keys(
    SHUTDOWN_PROMPT_ENGLISH
  ) as ShutdownPromptKey[]) {
    const line = (value as Record<string, unknown>)[key]
    if (typeof line === "string" && line.trim()) text[key] = line
  }
  return text
}
const blockerKeys = new Set([
  "game",
  "server",
  "install",
  "deletion",
  "task",
  "unconfirmed"
])
/** Work a quit, restart or update cancels as soon as it begins. */
const cancelledKinds = new Set(["install", "game", "task"])
/**
 * What a prompt for `view` says, as text keys: the renderer translates them
 * and the native dialog looks them up. Rows are one per kind: two servers
 * are one "a local server is still stopping". A `stopping` prompt only says
 * that the core is being ended, under the action's waiting title.
 */
export function shutdownPromptLines(view: ShutdownView) {
  const titleAction =
    view.action === "quit" || view.action === "update" ? view.action : "restart"
  if (view.stage === "stopping")
    return {
      title: `title_${titleAction}_waiting` as ShutdownPromptKey,
      body: "body_stopping" as ShutdownPromptKey,
      rows: [] as ShutdownPromptKey[],
      notes: [] as ShutdownPromptKey[],
      buttons: [] as { key: ShutdownPromptKey; choice: ShutdownChoice }[]
    }
  const body: ShutdownPromptKey =
    view.stage === "waiting"
      ? "body_waiting"
      : view.reason === "unresolved" || view.reason === "slow"
        ? "body_unresolved"
        : view.reason === "busy"
          ? "body_busy"
          : view.reason === "exit"
            ? "body_exit"
            : "body_unreachable"
  const rows = [
    ...new Set(
      view.blockers.map(
        (blocker) =>
          `blocker_${blockerKeys.has(blocker.kind) ? blocker.kind : "task"}` as ShutdownPromptKey
      )
    )
  ]
  // Too late to go back once the core is finishing, or agreed to stop and
  // only its exit was not seen yet.
  const canKeep = !view.finishing && view.reason !== "exit"
  const notes: ShutdownPromptKey[] = [
    ...(view.finishing ? (["too_late_to_keep"] as const) : []),
    ...(view.blockers.some((blocker) => blocker.kind === "deletion")
      ? (["deletion_warning"] as const)
      : []),
    "force_note",
    // The quit cancelled them as it began, and keeping GDLauncher open does
    // not start them again.
    ...(canKeep &&
    view.blockers.some((blocker) => cancelledKinds.has(blocker.kind))
      ? (["work_cancelled"] as const)
      : []),
    ...(view.serversStopped && canKeep ? (["servers_stopped"] as const) : []),
    ...(view.action === "quit" && view.stage === "waiting" && view.canWait
      ? (["wait_hint_quit"] as const)
      : [])
  ]
  const buttons: { key: ShutdownPromptKey; choice: ShutdownChoice }[] = [
    ...(view.canWait
      ? [
          {
            key: (view.stage === "waiting"
              ? "wait"
              : "try_again") as ShutdownPromptKey,
            choice: "wait" as const
          }
        ]
      : []),
    {
      key: `force_${view.action}` as ShutdownPromptKey,
      choice: "force"
    },
    ...(canKeep
      ? [{ key: "keep_open" as ShutdownPromptKey, choice: "keep" as const }]
      : [])
  ]
  return {
    title: `title_${titleAction}_${view.stage}` as ShutdownPromptKey,
    body,
    rows,
    notes,
    buttons
  }
}
/**
 * The dialog shown when no App can show the prompt: the loading screen, the
 * startup error screen after a crash, or a renderer that is reloading. It
 * uses the text the App last sent, in English until it has. Returns the
 * options for `dialog.showMessageBox` and the choice each button stands for.
 */
export function nativeShutdownDialog(
  view: ShutdownView,
  text: ShutdownPromptText = {}
) {
  const say = (key: ShutdownPromptKey) =>
    text[key] ?? SHUTDOWN_PROMPT_ENGLISH[key]
  const lines = shutdownPromptLines(view)
  const detail = [
    say(lines.body),
    lines.rows.map((row) => `• ${say(row)}`).join("\n"),
    ...lines.notes.map(say)
  ]
    .filter(Boolean)
    .join("\n\n")
  const choices = lines.buttons.map((button) => button.choice)
  // Escape keeps GDLauncher open, or else waits. Only a prompt whose sole
  // button is the force action (already finishing) maps it there.
  const cancelId = Math.max(
    choices.includes("keep")
      ? choices.indexOf("keep")
      : choices.indexOf("wait"),
    0
  )
  return {
    options: {
      type: view.stage === "blocked" ? ("warning" as const) : ("info" as const),
      title: say(lines.title),
      message: say(lines.title),
      detail,
      buttons: lines.buttons.map((button) => say(button.key)),
      defaultId: 0,
      cancelId,
      noLink: true
    },
    choices
  }
}
