interface NativeEmitter {
  on(event: string, listener: (...args: any[]) => void): unknown
  prependListener(event: string, listener: (...args: any[]) => void): unknown
  removeListener(event: string, listener: (...args: any[]) => void): unknown
}
export interface OwnedNativePreparation {
  pending(): boolean
  /** Resolves once preparation completed, failed or was disposed. */
  settled: Promise<void>
  dispose(): void
}

/** Mac's outer downloaded event starts native preparation, whose completion is
 * independent of the HTTP-serving promise. Observe only this owned boundary. */
export function createOwnedNativePreparation(updater: unknown) {
  const source = updater as {
    nativeUpdater?: NativeEmitter
    autoInstallOnAppQuit?: boolean
    squirrelDownloadedUpdate?: boolean
  }
  const errors = new WeakSet<object>()
  const ownsError = (error: unknown) =>
    typeof error === "object" && error !== null && errors.has(error)
  const rememberError = (error: unknown) => {
    if (typeof error === "object" && error !== null) errors.add(error)
  }
  return {
    ownsError,
    rememberError,
    watch(
      failed: (error: unknown) => void
    ): OwnedNativePreparation | undefined {
      const native = source.nativeUpdater
      if (
        !native ||
        source.autoInstallOnAppQuit !== true ||
        typeof source.squirrelDownloadedUpdate !== "boolean" ||
        typeof native.on !== "function" ||
        typeof native.prependListener !== "function" ||
        typeof native.removeListener !== "function"
      )
        return
      let live = true
      let settle!: () => void
      const settled = new Promise<void>((resolve) => {
        settle = resolve
      })
      const dispose = () => {
        if (!live) return
        live = false
        native.removeListener("error", error)
        native.removeListener("update-downloaded", completed)
        settle()
      }
      const error = (cause: unknown) => {
        if (!live) return
        rememberError(cause)
        dispose()
        failed(cause)
      }
      const completed = () => {
        if (live) dispose()
      }
      // Classify the exact native failure before the constructor forwards it
      // as a generic updater error; later forwards of that object stay fenced.
      native.prependListener("error", error)
      native.on("update-downloaded", completed)
      return { pending: () => live, settled, dispose }
    }
  }
}
