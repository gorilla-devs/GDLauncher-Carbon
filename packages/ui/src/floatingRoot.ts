// Floating UI (selects, menus, tooltips) mounts here so it never extends
// under the native ad view. Falls back to body where the host page has no
// #floating-root (showcase, website).
export function getFloatingRoot(): HTMLElement {
  if (typeof document === "undefined")
    return undefined as unknown as HTMLElement
  return document.getElementById("floating-root") ?? document.body
}
