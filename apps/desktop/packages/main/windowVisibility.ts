import type { BrowserWindow } from "electron"

// Tells the page when its window can't be seen, so theme motion can pause.
// `document.visibilityState` doesn't always change on minimise, so the page
// combines it with what main sends here.
export function forwardWindowVisibility(
  win: Pick<BrowserWindow, "on" | "isVisible" | "isMinimized">,
  send: (visible: boolean) => void
): void {
  const hidden = () => send(false)
  const shown = () => send(win.isVisible() && !win.isMinimized())
  win.on("minimize", hidden)
  win.on("hide", hidden)
  win.on("restore", shown)
  win.on("show", shown)
}
