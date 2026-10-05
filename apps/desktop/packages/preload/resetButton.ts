import type { RestoreOutcome } from "../main/utils/databaseRestore"
/** Both actual reset screens keep failures visible as text and permit retry. */
export function resetDatabaseClick(
  button: Pick<HTMLButtonElement, "disabled" | "textContent" | "style">,
  error: Pick<HTMLElement, "textContent">,
  invoke: () => Promise<RestoreOutcome>
) {
  return async () => {
    button.textContent = "Resetting..."
    button.disabled = true
    button.style.opacity = "0.6"
    error.textContent = ""
    try {
      const result = await invoke()
      if (result?.ok === true) return
      error.textContent =
        result?.ok === false
          ? result.message
          : "Reset did not complete. Please retry."
    } catch {
      error.textContent =
        "Reset did not complete. Current files and recovery copies must be preserved; check permissions and retry."
    }
    button.textContent = "Retry Reset"
    button.disabled = false
    button.style.opacity = "1"
  }
}
