import { JSX, Show, createSignal } from "solid-js"

interface Props {
  children: JSX.Element
  title?: string | JSX.Element
  size?: "standard" | "small"
  noPadding?: boolean
  defaultOpened?: boolean
  class?: string
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

  return (
    <div class="w-full box-border flex flex-col py-2 select-none max-w-full">
      <Show
        when={props.customHeader}
        fallback={
          <div
            class="h-8 flex items-center cursor-pointer press-effect active:scale-97"
            classList={{
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
                class="i-hugeicons:arrow-right-01 min-w-4 min-h-4 transition ease-spring text-lightSlate-500"
                classList={{
                  "rotate-90": opened()
                }}
              />
              <p
                class="m-0 text-lightSlate-500 flex items-center uppercase text-ellipsis max-w-full text-left"
                classList={{
                  "text-md": props.size !== "small",
                  "text-xs": props.size === "small"
                }}
              >
                {props.title}
              </p>
              <Show when={props.count !== undefined && props.count > 0}>
                <span class="bg-primary-500/20 text-primary-400 min-w-5 rounded-full px-1.5 py-0.5 text-center text-xs font-medium leading-none">
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
                class="i-hugeicons:cancel-01 h-3.5 w-3.5 text-lightSlate-700 hover:text-lightSlate-300 shrink-0 ml-1 transition-colors"
                onClick={(e) => {
                  e.stopPropagation()
                  props.onClear?.()
                }}
              />
            </Show>
          </div>
        }
      >
        {props.customHeader?.(toggle, opened)}
      </Show>
      <div
        class="grid transition-[grid-template-rows] duration-200 ease-in-out"
        style={{
          "grid-template-rows": opened() ? "1fr" : "0fr"
        }}
      >
        <div class="overflow-hidden pt-1">{props.children}</div>
      </div>
    </div>
  )
}

export { Collapsable }
