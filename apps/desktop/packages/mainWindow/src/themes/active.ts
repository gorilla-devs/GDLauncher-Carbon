import { createSignal } from "solid-js"
import type { FEResolvedTheme } from "@gd/core_module/bindings"

const [active, setActive] = createSignal<FEResolvedTheme | null>(null)
const [committed, setCommitted] = createSignal<FEResolvedTheme | null>(null)

/** The theme last put on the page; null before the first apply. */
export function activeTheme(): FEResolvedTheme | null {
  return active()
}

/**
 * The theme last applied for real: a hover preview leaves it alone, and the
 * leave that puts the saved theme back sets it again. What must not follow
 * the cursor (sound players, the layout) reads this.
 */
export function committedTheme(): FEResolvedTheme | null {
  return committed()
}

export function setActiveTheme(
  theme: FEResolvedTheme,
  opts: { preview?: boolean } = {}
): void {
  setActive(() => theme)
  if (!opts.preview) setCommitted(() => theme)
}
