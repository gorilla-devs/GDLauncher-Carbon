interface CloseWarningIpc {
  on(channel: string, callback: (...args: any[]) => void): unknown
  removeListener(channel: string, callback: (...args: any[]) => void): unknown
  send(channel: string, value: unknown): void
}
/** Why main held the close: what is running that closing affects. */
export interface CloseWarningReasons {
  game: boolean
  servers: boolean
  installs: boolean
}
/** Only the three known flags reach the page. A warning sent without them
 *  (an older main) is the game warning, as it was. */
export function closeWarningReasons(value: unknown): CloseWarningReasons {
  const reasons = (value ?? {}) as Record<string, unknown>
  const servers = reasons.servers === true
  const installs = reasons.installs === true
  return {
    game: reasons.game === true || (!servers && !installs),
    servers,
    installs
  }
}
// One ipcRenderer per document, so this counts the document's listeners.
const listeners = new WeakMap<CloseWarningIpc, number>()
/** Subscribes App to the close warning and tells main whether a listener is
 *  mounted, so main only holds a close open while the warning can be shown. */
export function subscribeCloseWarning(
  ipc: CloseWarningIpc,
  callback: (reasons: CloseWarningReasons) => void
): () => void {
  let active = true
  // The IPC event object stays in the preload.
  const listener = (_event: unknown, reasons: unknown) =>
    callback(closeWarningReasons(reasons))
  ipc.on("showAppCloseWarning", listener)
  const count = listeners.get(ipc) ?? 0
  listeners.set(ipc, count + 1)
  if (count === 0) ipc.send("close-warning-receiver", true)
  return () => {
    if (!active) return
    active = false
    ipc.removeListener("showAppCloseWarning", listener)
    const left = (listeners.get(ipc) ?? 1) - 1
    listeners.set(ipc, left)
    if (left === 0) ipc.send("close-warning-receiver", false)
  }
}
