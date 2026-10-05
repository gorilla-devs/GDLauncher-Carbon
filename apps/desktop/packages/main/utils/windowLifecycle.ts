export interface LauncherWindow {
  isDestroyed(): boolean
  close(): void
  destroy(): void
  hide(): void
  minimize(): void
  show(): void
  focus(): void
  webContents: { send(channel: string, ...args: unknown[]): void }
}
type Action =
  | "closeWindow"
  | "hideWindow"
  | "minimizeWindow"
  | "none"
  | "quitApp"
interface GameSession<W> {
  window: W | null
  action: Action
  closeState?: "pending" | "closed" | "canceled"
  exited?: boolean
}
/** Aggregate snapshots never apply launch actions, particularly for adopted games. */
export class LauncherLifecycle<W extends LauncherWindow> {
  private revision = -1
  private coreSession = ""
  private coreEnded = false
  private running = new Set<number>()
  private sessions = new Map<string, GameSession<W>>()
  private closeIntent: W | null = null
  private closeEvent: { defaultPrevented?: boolean } | null = null
  private quitting = false
  /** Renderers whose App has mounted the close-warning listener. */
  private warningReceivers = new WeakSet<object>()
  constructor(
    private readonly io: {
      current(): W | null
      clear(window: W): void
      create(): unknown
      quit(): void
    }
  ) {}
  beginCore(session = "") {
    this.coreSession = session
    this.coreEnded = false
    this.revision = -1
    this.running.clear()
    this.sessions.clear()
  }
  /** The core process exited. It can no longer report a game's exit, so
   *  nothing it reported may keep arming the close warning or keep the
   *  launcher alive without a window; records it printed before exiting
   *  that are read afterwards are ignored.
   *
   *  Returns whether its Close Window games were what kept the launcher from
   *  quitting once windowless. A window may still be open (a relaunch during
   *  the game), so callers must check for one before quitting. When none is,
   *  'window-all-closed' already deferred the quit for those games and does
   *  not run again, so the caller makes that decision. */
  endCore(session = this.coreSession): boolean {
    if (session !== this.coreSession) return false
    const keptAlive = this.keepAliveWithoutWindows()
    this.coreEnded = true
    this.revision = -1
    this.running.clear()
    this.sessions.clear()
    return keptAlive
  }
  get gameRunning() {
    return this.running.size > 0
  }
  snapshot(payload: string, session = this.coreSession) {
    if (session !== this.coreSession || this.coreEnded) return
    try {
      const value = JSON.parse(payload)
      if (
        !Number.isSafeInteger(value.revision) ||
        value.revision <= this.revision ||
        !Array.isArray(value.instances) ||
        value.instances.some((id: unknown) => !Number.isSafeInteger(id))
      )
        return
      this.revision = value.revision
      this.running = new Set(value.instances)
    } catch {
      /* Malformed notifications cannot change current authority. */
    }
  }
  event(payload: string, session = this.coreSession) {
    if (session !== this.coreSession || this.coreEnded) return
    const [event, id, task, action] = payload.split("|")
    if (
      !/^\d+$/.test(id ?? "") ||
      !/^\d+$/.test(task ?? "") ||
      ![
        "closeWindow",
        "hideWindow",
        "minimizeWindow",
        "none",
        "quitApp"
      ].includes(action)
    )
      return
    const key = `${id}|${task}`
    if (event === "GAME_LAUNCHED") {
      if (this.sessions.has(key)) return
      const window = this.io.current()
      const owned: GameSession<W> = { window, action: action as Action }
      this.sessions.set(key, owned)
      if (action === "quitApp") {
        this.confirmQuit()
        return
      }
      if (!window || window.isDestroyed()) return
      if (action === "closeWindow") {
        this.closeIntent = window
        owned.closeState = "pending"
        this.closeEvent = null
        try {
          window.close()
        } finally {
          if (
            (this.closeEvent as { defaultPrevented?: boolean } | null)
              ?.defaultPrevented
          )
            this.cancelLaunchClose(window)
          this.closeIntent = null
          this.closeEvent = null
        }
      } else if (action === "hideWindow") window.hide()
      else if (action === "minimizeWindow") window.minimize()
    } else if (event === "GAME_CLOSED") {
      const session = this.sessions.get(key)
      if (!session) return
      session.exited = true
      // Accepted closure and matching exit can arrive in either order. Keep
      // this captured purpose until actual closed (or cancellation) settles it.
      if (session.closeState === "pending") return
      this.sessions.delete(key)
      // A game that exits during a quit does not bring the launcher back:
      // the quit shows its own window if it needs the user, and Keep
      // GDLauncher open shows it again.
      if (this.quitting || this.quitPending) return
      if (session.action === "closeWindow") {
        if (session.closeState === "canceled") return
        const current = this.io.current()
        if (!current || current.isDestroyed()) this.io.create()
      } else if (
        session.action === "hideWindow" ||
        session.action === "minimizeWindow"
      ) {
        const current = this.io.current()
        if (current === session.window && !current?.isDestroyed()) {
          current?.show()
          current?.focus()
        } else if (!current || current.isDestroyed()) this.io.create()
      }
    }
  }
  close(
    window: W,
    event: { preventDefault(): void; defaultPrevented?: boolean },
    warning: boolean
  ) {
    if (this.closeIntent === window) {
      this.closeEvent = event
      return
    }
    if (this.quitting || !this.gameRunning || !warning) return
    // Only a mounted App can show the warning. Before it mounts (intro,
    // loading, the startup error screen) or after its document is replaced,
    // holding the close would make X silently do nothing, so the close goes
    // ahead as it did before adopted games counted as running.
    const contents = window.webContents
    if (!this.warningReceivers.has(contents)) return
    event.preventDefault()
    contents.send("showAppCloseWarning")
  }
  /**
   * Whether a close that went ahead (the warning did not hold it) would quit
   * GDLauncher: it is not a Close Window launch, no quit was approved yet,
   * `window` is the last window and no Close Window game keeps the launcher
   * running without one. The caller then hides the window instead of
   * letting it be destroyed and quits, so a quit that cannot finish still
   * has a window to explain why in and to come back to.
   */
  quitsOnClose(
    window: W,
    event: { defaultPrevented?: boolean },
    otherWindows: number
  ) {
    return (
      !event.defaultPrevented &&
      this.closeIntent !== window &&
      !this.quitting &&
      otherWindows === 0 &&
      !this.keepAliveWithoutWindows()
    )
  }
  /** Records whether `contents`' App currently listens for the close warning.
   *
   *  Keyed by WebContents, not by document. A reload or crash withdraws the
   *  old App's acknowledgement (did-navigate, render-process-gone) and the
   *  new App sends its own once it mounts. An old document never withdraws
   *  after that: the renderer does not dispose App when its page unloads, so
   *  App's cleanup only runs while its own document is current. */
  warningReceiver(contents: object, ready: boolean) {
    if (ready) this.warningReceivers.add(contents)
    else this.warningReceivers.delete(contents)
  }
  closed(window: W) {
    const wasCurrent = this.io.current() === window
    this.io.clear(window)
    let reopen = false,
      launchClose = false
    for (const [key, session] of this.sessions) {
      if (session.window !== window || session.closeState !== "pending")
        continue
      launchClose = true
      session.closeState = "closed"
      if (session.exited) {
        this.sessions.delete(key)
        reopen = true
      }
    }
    // The user closed the window shown while a Close Window game ran (a
    // relaunch, or the reopen for another game). That closes the launcher:
    // those games stop waiting to reopen it, so it quits like any other
    // close instead of running unseen and popping back when a game exits.
    if (wasCurrent && !launchClose)
      for (const [key, session] of this.sessions)
        if (session.closeState === "closed") this.sessions.delete(key)
    const current = this.io.current()
    if (reopen && !this.quitting && (!current || current.isDestroyed()))
      this.io.create()
  }
  cancelLaunchClose(window: W) {
    for (const [key, session] of this.sessions) {
      if (session.window !== window || session.closeState !== "pending")
        continue
      session.closeState = "canceled"
      if (session.exited) this.sessions.delete(key)
    }
  }
  keepAliveWithoutWindows() {
    return (
      !this.quitting &&
      [...this.sessions.values()].some(
        (session) =>
          session.closeState === "pending" || session.closeState === "closed"
      )
    )
  }
  confirmQuit() {
    if (this.quitting) return
    this.quitting = true
    this.io.quit()
  }
  quitFailed() {
    this.quitting = false
  }
  approveQuit() {
    this.quitting = true
  }
}
/**
 * The windows that mean GDLauncher still has a window: its own, shown or
 * hidden, and any other window the user can see.
 *
 * ow-electron opens hidden 1x1 windows of its own: the Overwolf consent
 * window for about 10 s after every start outside the EU, and on Windows the
 * package manager's for the whole session. They show nothing,
 * 'window-all-closed' already ignores them and they close when GDLauncher
 * quits, so they must not keep a close from quitting or a launcher without a
 * window running.
 */
export function userWindows<
  W extends { isDestroyed(): boolean; isVisible(): boolean }
>(windows: readonly W[], own: W | null): W[] {
  return windows.filter(
    (window) => !window.isDestroyed() && (window === own || window.isVisible())
  )
}
/**
 * Body of the main window's 'closed' listener.
 *
 * Electron marks a window destroyed before it emits 'closed', so inside the
 * listener every getter of the window, `webContents` included, throws "Object
 * has been destroyed". Only the window's identity may be used here, and
 * `contents` must be the WebContents captured when the window was created.
 *
 * Per-window state is released first, and only when the closed window is the
 * current one, so a window that `lifecycle.closed` reopens starts clean and a
 * replacement that already exists keeps its own state.
 */
export function mainWindowClosed<W, C>(
  window: W,
  contents: C,
  io: {
    current(): W | null
    release(): void
    lifecycle: { closed(window: W): void }
    protocol: { close(contents: C): void }
  }
) {
  const current = io.current()
  if (!current || current === window) io.release()
  try {
    io.lifecycle.closed(window)
  } finally {
    // A window reopened above has already rebound the queue to its own
    // contents; closing the old sender is then a no-op.
    io.protocol.close(contents)
  }
}
