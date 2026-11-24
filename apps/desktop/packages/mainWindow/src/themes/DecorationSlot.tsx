import { For, Show, JSX } from "solid-js"
import { activeTheme } from "./active"
import {
  DecorationAnchor,
  hasDecorations,
  slotClass,
  slotDecorations
} from "./decorations"

/** True when the active theme decorates `anchor` for `itemKey`. Anchors that
 * need `relative isolate` only add it then, so other themes are unchanged. */
export const isDecorated = (anchor: DecorationAnchor, itemKey?: string) =>
  hasDecorations(activeTheme(), anchor, itemKey ?? anchor)

// Rendered by the app inside each anchor (for `panel`, as a sibling over the
// panel box). Decorations are static images; nothing here runs per frame.
// The slot is `z-[1]`, so the parent must be positioned and `isolate`d to keep
// that z-index from leaking. The nav is not isolated, so its slot passes
// `!z-0` and stays in the nav's own stacking order, under the effects layer.
// Pass overrides to the default `inset-0` with important classes (`!left-4`).
export function DecorationSlot(props: {
  anchor: DecorationAnchor
  itemKey?: string
  class?: string
}): JSX.Element {
  const items = () =>
    slotDecorations(activeTheme(), props.anchor, props.itemKey ?? props.anchor)
  return (
    <Show when={items().length > 0}>
      <div
        data-gdl-decorations={props.anchor}
        aria-hidden="true"
        class={slotClass(props.anchor, props.class)}
      >
        <For each={items()}>
          {(it) => <span data-gdl-decoration style={it.style} />}
        </For>
      </div>
    </Show>
  )
}
