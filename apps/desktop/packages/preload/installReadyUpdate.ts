/** Capture displayed authority before asynchronous IPC. An obsolete rejection
 * cannot replace a newer update's presentation. */
export function installReadyUpdateAction(
  readyId: () => number | undefined,
  install: (id: number) => Promise<unknown>,
  failed: (message: string) => void
) {
  return async () => {
    const captured = readyId()
    if (captured === undefined) return
    try {
      await install(captured)
    } catch (error) {
      if (readyId() === captured)
        failed(
          error instanceof Error
            ? error.message
            : "Update installation failed. Check for updates and retry."
        )
    }
  }
}
