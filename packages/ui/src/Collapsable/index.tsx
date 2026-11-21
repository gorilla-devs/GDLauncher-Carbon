import { JSX, Show, children, createSignal } from "solid-js"
import { gdlHooks, withStates } from "../hooks"

interface Props {
  children: JSX.Element
  title?: string | JSX.Element
  size?: "standard" | "small"
  noPadding?: boolean
  defaultOpened?: boolean
  class?: string
  "data-gdl"?: string
  /** Theme hook tokens for the default header row (not the custom header). */
  headerGdl?: string
  /** Rendered as the default header row's last child; the row is then positioned and isolated (an empty slot changes nothing). */
  headerSlot?: JSX.Element
  /** Rendered as the root's last child; the root is then positioned and isolated (an empty slot changes nothing). */
  slot?: JSX.Element
  /** Number of active items in this section */
  count?: number
  /** Callback to clear this section's filters */
  onClear?: () => void
  /** Custom header render function - receives toggle function and opened state */
  customHeader?: (toggle: () => void, isOpened: () => boolean) => JSX.Element
}

const Collapsable = (props: Props) => {
  const [opened, setOpened] = createSignal(props.defaultOpened ?? true)

  const toggle = () => setOpened((prev) => !prev)

  // Resolved once, so an empty slot (a decoration slot with nothing to draw
  // renders nothing) leaves the row and root unpositioned.
  const headerSlot = children(() => props.headerSlot)
  const slot = children(() => props.slot)

  return (
    <div
      data-gdl={gdlHooks(
        withStates("collapsable", { open: opened() }),
        props["data-gdl"]
      )}
      class="w-full box-border flex flex-col py-2 select-none max-w-full"
      classList={{ "relative isolate": !!slot() }}
    >
      <Show
        when={props.customHeader}
        fallback={
          // A real button, so the section opens from the keyboard too.
          <button
            type="button"
            aria-expanded={opened()}
            data-gdl={gdlHooks("collapsable.header", props.headerGdl)}
            class="h-8 w-full flex items-center text-left cursor-pointer press-effect active:scale-97 rounded-md outline-none focus-visible:outline focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-primary-500"
            classList={{
              "relative isolate": !!headerSlot(),
              "px-6": props.size !== "small" && !props.noPadding,
              "px-2": props.size === "small" && !props.noPadding,
              ...(props.class && {
                [props.class]: true
              })
            }}
            onClick={toggle}
          >
            <div class="flex items-center gap-2 flex-1 min-w-0">
              <div
                data-gdl="collapsable.chevron"
                class="i-hugeicons:arrow-right-01 min-w-4 min-h-4 transition ease-spring text-lightSlate-500"
                classList={{
                  "rotate-90": opened()
                }}
              />
              <p
                data-gdl="collapsable.title"
                class="m-0 text-lightSlate-500 flex items-center uppercase text-ellipsis max-w-full text-left"
                classList={{
                  "text-md": props.size !== "small",
                  "text-xs": props.size === "small"
                }}
              >
                {props.title}
              </p>
              <Show when={props.count !== undefined && props.count > 0}>
                <span
                  data-gdl="collapsable.count"
                  class="bg-primary-500/20 text-primary-400 min-w-5 rounded-full px-1.5 py-0.5 text-center text-xs font-medium leading-none"
                >
                  {props.count}
                </span>
              </Show>
            </div>
            <Show
              when={
                props.onClear && props.count !== undefined && props.count > 0
              }
            >
              <div
                data-gdl="collapsable.clear"
                class="i-hugeicons:cancel-01 h-3.5 w-3.5 text-lightSlate-700 hover:text-lightSlate-300 shrink-0 ml-1 transition-colors"
                onClick={(e) => {
                  e.stopPropagation()
                  props.onClear?.()
                }}
              />
            </Show>
            {headerSlot()}
          </button>
        }
      >
        {props.customHeader?.(toggle, opened)}
      </Show>
      <div
        class="grid transition-[grid-template-rows] duration-200 ease-in-out"
        style={{
          "grid-template-rows": opened() ? "1fr" : "0fr",
          // One column no wider than the section: a long, truncated label
          // inside must not widen it (a grid item is as wide as its content
          // by default).
          "grid-template-columns": "minmax(0, 1fr)"
        }}
      >
        {/* Closed, nothing inside can take focus: Tab skips it rather than
            scrolling a sliver of it into view. Its clipping is in style.css,
            so a theme can let it spill; `min-h-0` keeps the opening
            animation, which `clip` alone would skip. */}
        <div
          data-gdl="collapsable.body"
          class="min-h-0 min-w-0 pt-1"
          inert={!opened()}
        >
          {props.children}
        </div>
      </div>
      {slot()}
    </div>
  )
}

export { Collapsable }
