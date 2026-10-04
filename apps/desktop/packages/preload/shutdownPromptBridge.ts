interface ShutdownPromptIpc {
  on(
    channel: string,
    callback: (event: unknown, message: unknown) => void
  ): unknown
  removeListener(
    channel: string,
    callback: (event: unknown, message: unknown) => void
  ): unknown
  send(channel: string, value: unknown): void
}
// One ipcRenderer per document, so this counts the document's listeners.
const listeners = new WeakMap<ShutdownPromptIpc, number>()
/**
 * Subscribes App to the prompt shown when a quit, restart or update does not
 * finish, and tells main whether a listener is mounted: without one, main
 * asks with a native dialog instead.
 */
export function subscribeShutdownPrompt(
  ipc: ShutdownPromptIpc,
  callback: (message: unknown) => void
): () => void {
  let active = true
  // The IPC event object stays in the preload.
  const listener = (_event: unknown, message: unknown) => callback(message)
  ipc.on("shutdown-prompt", listener)
  const count = listeners.get(ipc) ?? 0
  listeners.set(ipc, count + 1)
  if (count === 0) ipc.send("shutdown-prompt-receiver", true)
  return () => {
    if (!active) return
    active = false
    ipc.removeListener("shutdown-prompt", listener)
    const left = (listeners.get(ipc) ?? 1) - 1
    listeners.set(ipc, left)
    if (left === 0) ipc.send("shutdown-prompt-receiver", false)
  }
}
