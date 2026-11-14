interface CoreExitIpc {
  on(channel: string, callback: (...args: any[]) => void): unknown
  removeListener(channel: string, callback: (...args: any[]) => void): unknown
}
/** How the core ended: an exit code, or the signal that ended it. */
export interface CoreExit {
  code: number | null
  signal: string | null
}
/** Only an exit main reported reaches the page: anything else is none. */
export function coreExit(value: unknown): CoreExit | null {
  if (typeof value !== "object" || value === null) return null
  const { code, signal } = value as Record<string, unknown>
  return {
    code: typeof code === "number" ? code : null,
    signal: typeof signal === "string" ? signal : null
  }
}
/** Tells the page when the core stopped by itself. */
export function subscribeCoreExit(
  ipc: CoreExitIpc,
  callback: (exit: CoreExit) => void
): () => void {
  // The IPC event object stays in the preload.
  const listener = (_event: unknown, value: unknown) => {
    const exit = coreExit(value)
    if (exit) callback(exit)
  }
  ipc.on("coreExited", listener)
  return () => void ipc.removeListener("coreExited", listener)
}
