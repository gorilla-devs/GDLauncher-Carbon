/** How the core ended: an exit code, or the signal that ended it. */
export interface CoreExit {
  code: number | null
  signal: string | null
}

/**
 * How long an exit is held before the window is told. The core is in the
 * launcher's process group, so Ctrl+C, a logout or a kill of the whole tree
 * can end it a moment before the launcher's own quit begins.
 */
export const CORE_EXIT_SETTLE_MS = 1000

/**
 * Decides whether the window is told that the core stopped by itself: it
 * stays on a core that answers nothing, and only a restart helps.
 *
 * Nothing is said when the core never became ready (the startup error screen
 * covers that), or while `expected` holds: something is stopping the core on
 * purpose, or GDLauncher or the session is ending. `expected` is asked when
 * the core exits and again after the wait, and once more for each window
 * that loads later.
 */
export function createCoreExitNotice(io: {
  expected: () => boolean
  send: (exit: CoreExit) => void
  wait?: (run: () => void, ms: number) => void
}) {
  const wait =
    io.wait ?? ((run: () => void, ms: number) => void setTimeout(run, ms))
  let stopped: CoreExit | undefined
  return {
    exited(ready: boolean, exit: CoreExit) {
      if (!ready || io.expected()) return
      wait(() => {
        if (io.expected()) return
        stopped = exit
        io.send(exit)
      }, CORE_EXIT_SETTLE_MS)
    },
    /** For a window that loads after the exit, such as a reload. */
    get current(): CoreExit | null {
      return stopped && !io.expected() ? stopped : null
    }
  }
}
