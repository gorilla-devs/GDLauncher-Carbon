import { isSupportedProtocol } from "./protocolQueue"

/**
 * The arguments for restarting a launcher started with `argv`: the same,
 * less each deep link that is no longer `pending`, so a link the App has
 * acted on is not acted on again. A link still pending (the App never
 * mounted, as on the startup error screen) is kept. Null when nothing is
 * left out.
 */
export function relaunchArgs(
  argv: readonly string[],
  pending: readonly string[]
) {
  const args = argv.slice(1)
  const kept = args.filter(
    (arg) => !isSupportedProtocol(arg) || pending.includes(arg)
  )
  return kept.length === args.length ? null : kept
}

/**
 * Makes app.relaunch() with no options restart without the links
 * relaunchArgs leaves out. Without options Electron restarts with this
 * process's own argv, and main queues every link in argv on each start, so
 * every in-app restart, and electron-updater's after a deb or rpm install,
 * reopened the link that started the launcher. A call with options, or with
 * nothing to leave out, keeps Electron's own behaviour.
 */
export function relaunchWithoutHandledLinks(
  app: { relaunch(options?: { args?: string[]; execPath?: string }): void },
  argv: () => readonly string[],
  pending: () => readonly string[]
) {
  const relaunch = app.relaunch.bind(app)
  app.relaunch = (options) => {
    if (options) return relaunch(options)
    const args = relaunchArgs(argv(), pending())
    if (args) relaunch({ args })
    else relaunch()
  }
}
