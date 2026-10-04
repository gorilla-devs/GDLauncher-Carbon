/**
 * The startup error screen's Restart. A restart that does not finish asks
 * the user first (main/utils/shutdownFlow), so this only reports one that
 * failed outright; keeping GDLauncher open just enables the button again.
 */
export function restartClick(
  button: Pick<HTMLButtonElement, "disabled" | "style">,
  error: Pick<HTMLElement, "textContent">,
  invoke: () => Promise<unknown>
) {
  return async () => {
    if (button.disabled) return
    button.disabled = true
    button.style.opacity = "0.6"
    error.textContent = ""
    try {
      await invoke()
    } catch {
      error.textContent =
        "GDLauncher could not restart. Close it and open it again."
    }
    button.disabled = false
    button.style.opacity = "1"
  }
}
