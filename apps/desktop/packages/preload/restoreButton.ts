import type { RestoreOutcome } from "../main/utils/databaseRestore"
/** Success remains busy through relaunch; failures are safe text and retryable. */
export function restoreSnapshotClick(
  button: Pick<HTMLButtonElement, "disabled" | "textContent" | "style">,
  error: Pick<HTMLElement, "textContent">,
  invoke: () => Promise<RestoreOutcome>
) {
  return async () => {
    button.textContent = "Restoring..."
    button.disabled = true
    button.style.opacity = "0.6"
    error.textContent = ""
    try {
      const result = await invoke()
      if (result?.ok === true) return
      error.textContent =
        result?.ok === false
          ? result.message
          : "Restore did not complete. Please retry."
    } catch {
      error.textContent =
        "Restore did not complete. Check the snapshot and retry."
    }
    button.textContent = "Retry Restore"
    button.disabled = false
    button.style.opacity = "1"
  }
}
