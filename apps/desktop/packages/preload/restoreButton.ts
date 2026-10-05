import type {
  RestoreOutcome,
  RestoreRecoveryOutcome
} from "../main/utils/databaseRestore"
/** The same for Recover Database on the startup screen of an interrupted restore. */
export function recoverRestoreClick(
  button: Pick<HTMLButtonElement, "disabled" | "textContent" | "style">,
  error: Pick<HTMLElement, "textContent">,
  invoke: () => Promise<RestoreRecoveryOutcome>
) {
  return async () => {
    button.textContent = "Recovering..."
    button.disabled = true
    button.style.opacity = "0.6"
    error.textContent = ""
    try {
      const result = await invoke()
      if (result?.ok === true) return
      error.textContent =
        result?.ok === false
          ? result.message
          : "Recovery did not complete. Please retry."
    } catch {
      error.textContent = "Recovery did not complete. Please retry."
    }
    button.textContent = "Retry Recover"
    button.disabled = false
    button.style.opacity = "1"
  }
}
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
