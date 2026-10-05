interface CloseWarningIpc {
  on(channel: string, callback: () => void): unknown
  removeListener(channel: string, callback: () => void): unknown
  send(channel: string, value: unknown): void
}
// One ipcRenderer per document, so this counts the document's listeners.
const listeners = new WeakMap<CloseWarningIpc, number>()
/** Subscribes App to the close warning and tells main whether a listener is
 *  mounted, so main only holds a close open while the warning can be shown. */
export function subscribeCloseWarning(
  ipc: CloseWarningIpc,
  callback: () => void
): () => void {
  let active = true
  // The IPC event object stays in the preload.
  const listener = () => callback()
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
