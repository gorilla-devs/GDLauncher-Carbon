type Listener = (...args: any[]) => void
interface NativeEmitter {
  listeners(event: string): Listener[]
  removeListener(event: string, listener: Listener): unknown
}

/** Own only the queued completion callbacks attached by this native dispatch. */
export function createOwnedInstallDispatch(updater: {
  quitAndInstall(silent: boolean, run: boolean): void
}) {
  return () => {
    const native = (updater as unknown as { nativeUpdater?: NativeEmitter })
      .nativeUpdater
    const previous = new Set(native?.listeners("update-downloaded") ?? [])
    const capture = () => {
      const attached =
        native
          ?.listeners("update-downloaded")
          .filter((listener) => !previous.has(listener)) ?? []
      let disposed = false
      return () => {
        if (disposed) return
        disposed = true
        for (const listener of attached)
          native!.removeListener("update-downloaded", listener)
      }
    }
    try {
      updater.quitAndInstall(true, true)
    } catch (error) {
      capture()()
      throw error
    }
    return capture()
  }
}
