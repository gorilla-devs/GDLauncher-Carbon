import { createSignal } from "solid-js"

// Module-level signal for tracking which folder is being animated (like clickedInstanceId pattern)
export const [clickedFolderId, setClickedFolderId] = createSignal<
  number | null
>(null)

// Track which instance indices are visible in the expanded folder viewport
export const [visibleFolderIndices, setVisibleFolderIndices] = createSignal<
  number[]
>([])

// Module-level style element for view transition CSS injection
// Only one folder animation can run at a time (by design)
let dynamicStyleElement: HTMLStyleElement | null = null

export function injectFolderTransitionCSS(
  indices: number[],
  direction: "open" | "close"
) {
  // Remove existing dynamic styles
  if (dynamicStyleElement) {
    dynamicStyleElement.remove()
  }

  // Direction-specific CSS for folder-tile old/new snapshots
  const folderTileCSS =
    direction === "close"
      ? `
      /* On close: keep old (expanded) visible while morphing */
      ::view-transition-old(folder-tile) {
        opacity: 1 !important;
      }
      ::view-transition-new(folder-tile) {
        opacity: 0;
      }
    `
      : `
      /* On open: fade out old (collapsed), show new (expanded) */
      ::view-transition-old(folder-tile) {
        opacity: 0;
      }
      ::view-transition-new(folder-tile) {
        opacity: 1;
      }
    `

  // Keyframes for empty slot fade-in during close animation
  const emptySlotKeyframes =
    direction === "close"
      ? `
    @keyframes fadeInEmptySlot {
      from { opacity: 0; }
      to { opacity: 1; }
    }
  `
      : ""

  // Preview-specific CSS (only when there are preview indices)
  let previewCSS = ""
  if (indices.length > 0) {
    const groupSelectors = indices
      .map((i) => `::view-transition-group(folder-preview-${i})`)
      .join(",\n")
    const oldNewSelectors = indices
      .flatMap((i) => [
        `::view-transition-old(folder-preview-${i})`,
        `::view-transition-new(folder-preview-${i})`
      ])
      .join(",\n")

    previewCSS = `
      ${groupSelectors} {
        animation-duration: 300ms;
        animation-timing-function: cubic-bezier(0.32, 0.72, 0, 1);
        z-index: 1;
      }
      ${oldNewSelectors} {
        animation-duration: 300ms;
        animation-timing-function: cubic-bezier(0.32, 0.72, 0, 1);
        mix-blend-mode: normal;
      }
    `
  }

  const css = `
    ${emptySlotKeyframes}
    ${previewCSS}
    ${folderTileCSS}
    ::view-transition-group(folder-name) {
      animation-duration: 300ms;
      animation-timing-function: cubic-bezier(0.32, 0.72, 0, 1);
      z-index: 2;
    }
    ::view-transition-old(folder-name),
    ::view-transition-new(folder-name) {
      animation-duration: 300ms;
      animation-timing-function: cubic-bezier(0.32, 0.72, 0, 1);
      mix-blend-mode: normal;
    }
  `

  dynamicStyleElement = document.createElement("style")
  dynamicStyleElement.textContent = css
  document.head.appendChild(dynamicStyleElement)
}

export function removeFolderTransitionCSS() {
  if (dynamicStyleElement) {
    dynamicStyleElement.remove()
    dynamicStyleElement = null
  }
}
