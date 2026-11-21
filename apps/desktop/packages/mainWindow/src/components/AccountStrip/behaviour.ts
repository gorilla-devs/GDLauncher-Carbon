import type { MotionState } from "@/themes/motion"

/** How long the strip takes to open and to close, and its easings: it opens
 *  quickly and settles, and closes evenly, so its shrinking shows. The top
 *  bar's other buttons fold away and back on the same timing. */
export const STRIP_OPEN_MS = 300
export const STRIP_CLOSE_MS = 260
export const STRIP_OPEN_EASE = "cubic-bezier(0.2, 0.8, 0.2, 1)"
export const STRIP_CLOSE_EASE = "cubic-bezier(0.4, 0, 0.2, 1)"

/**
 * While the strip is open: Esc in it closes it and gives focus back to the
 * account button; focus leaving it, or a press anywhere else, closes it and
 * leaves focus where it went. Esc anywhere else is left to whatever has
 * focus. Returns the function that stops listening.
 */
export function watchOpenStrip(
  root: HTMLElement,
  close: (returnFocus: boolean) => void,
  doc: Document = document
): () => void {
  const onKeyDown = (e: KeyboardEvent) => {
    if (e.key !== "Escape") return
    const active = doc.activeElement
    if (active && active !== doc.body && !root.contains(active)) return
    e.preventDefault()
    e.stopPropagation()
    close(true)
  }
  const onPointerDown = (e: Event) => {
    if (!root.contains(e.target as Node)) close(false)
  }
  const onFocusOut = (e: FocusEvent) => {
    const next = e.relatedTarget as Node | null
    if (next && !root.contains(next)) close(false)
  }
  doc.addEventListener("keydown", onKeyDown, true)
  doc.addEventListener("pointerdown", onPointerDown, true)
  root.addEventListener("focusout", onFocusOut)
  return () => {
    doc.removeEventListener("keydown", onKeyDown, true)
    doc.removeEventListener("pointerdown", onPointerDown, true)
    root.removeEventListener("focusout", onFocusOut)
  }
}

/** Where "+" goes: the add-account flow, which comes back to this page. */
export function addAccountPath(location: {
  pathname: string
  search: string
}): string {
  const back = encodeURIComponent(`${location.pathname}${location.search}`)
  return `/?addMicrosoftAccount=true&returnTo=${back}`
}

/** The strip's opening animation plays only while motion runs: never with
 *  reduced motion (the app's setting or the system's) or a hidden window. */
export function animatesOpen(motion: MotionState): boolean {
  return motion === "run"
}
