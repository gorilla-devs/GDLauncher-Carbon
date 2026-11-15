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

/**
 * How long the confirming button ignores clicks after it appears, so the
 * second click of a double click on the first button cannot land on it.
 */
export const RESET_CONFIRM_GUARD_MS = 600

/**
 * A reset takes two clicks on both reset screens. The first turns `button`
 * into Cancel, in the same place and at least as wide, and shows `confirm`;
 * only `confirm` resets. A failed reset has to be confirmed again.
 */
export function confirmedResetClick(io: {
  button: HTMLButtonElement
  confirm: HTMLButtonElement
  error: Pick<HTMLElement, "textContent">
  invoke: () => Promise<RestoreOutcome>
  /** Controls beside the button that give their room to `confirm`. */
  hide?: HTMLElement[]
  now?: () => number
}) {
  const now = io.now ?? (() => Date.now())
  const reset = resetDatabaseClick(io.button, io.error, io.invoke)
  let armed:
    | { at: number; label: string | null; style: Record<string, string> }
    | undefined

  const disarm = () => {
    if (!armed) return
    io.button.textContent = armed.label
    Object.assign(io.button.style, armed.style)
    io.confirm.style.display = "none"
    for (const element of io.hide ?? []) element.style.display = ""
    armed = undefined
  }

  io.confirm.style.display = "none"
  io.button.addEventListener("click", () => {
    if (armed) return disarm()
    const { background, color, border } = io.button.style
    armed = {
      at: now(),
      label: io.button.textContent,
      style: { background, color, border }
    }
    io.button.style.minWidth = `${io.button.offsetWidth}px`
    io.button.textContent = "Cancel"
    io.button.style.background = "transparent"
    io.button.style.color = "rgb(var(--lightSlate-300))"
    io.button.style.border = "1px solid rgb(var(--darkSlate-500))"
    for (const element of io.hide ?? []) element.style.display = "none"
    io.confirm.style.display = ""
  })
  io.confirm.addEventListener("click", () => {
    if (!armed || now() - armed.at < RESET_CONFIRM_GUARD_MS) return
    disarm()
    void reset()
  })
}

/**
 * The reset of a screen whose first choice is something else: `open` shows
 * `panel`, which holds the warning, `confirm` and `cancel`, in place of the
 * `hide` elements. Only `confirm` resets, and it stays for a retry when the
 * reset fails.
 */
export function revealedResetClick(io: {
  open: HTMLButtonElement
  panel: HTMLElement
  confirm: HTMLButtonElement
  cancel: HTMLButtonElement
  error: Pick<HTMLElement, "textContent">
  invoke: () => Promise<RestoreOutcome>
  hide: HTMLElement[]
  now?: () => number
}) {
  const now = io.now ?? (() => Date.now())
  const reset = resetDatabaseClick(io.confirm, io.error, io.invoke)
  let shownAt: number | undefined
  const displays = io.hide.map((element) => element.style.display)
  const show = (shown: boolean) => {
    shownAt = shown ? now() : undefined
    io.panel.style.display = shown ? "flex" : "none"
    io.hide.forEach((element, index) => {
      element.style.display = shown ? "none" : displays[index]
    })
  }

  io.panel.style.display = "none"
  io.open.addEventListener("click", () => show(true))
  io.cancel.addEventListener("click", () => {
    show(false)
  })
  io.confirm.addEventListener("click", async () => {
    if (shownAt === undefined || now() - shownAt < RESET_CONFIRM_GUARD_MS)
      return
    io.cancel.disabled = true
    io.cancel.style.opacity = "0.6"
    await reset()
    // A reset that worked leaves both busy until the restart.
    if (io.confirm.disabled) return
    io.cancel.disabled = false
    io.cancel.style.opacity = "1"
  })
}
