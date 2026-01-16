import { JSX, createSignal } from "solid-js"

interface Props {
  children: JSX.Element
  title?: string | JSX.Element
  size?: "standard" | "small"
  noPadding?: boolean
  defaultOpened?: boolean
  class?: string
}

const Collapsable = (props: Props) => {
  const [opened, setOpened] = createSignal(props.defaultOpened ?? true)

  return (
    <div class="w-full box-border flex flex-col py-2 select-none max-w-full">
      <div
        class="w-fit h-8 flex gap-2 items-center cursor-pointer press-effect active:scale-97"
        classList={{
          "px-6": props.size !== "small" && !props.noPadding,
          "px-2": props.size === "small" && !props.noPadding,
          ...(props.class && {
            [props.class]: true
          })
        }}
        onClick={() => {
          setOpened((prev) => !prev)
        }}
      >
        <div
          class="i-hugeicons:arrow-right-01 min-w-4 min-h-4 transition ease-spring text-lightSlate-700"
          classList={{
            "rotate-90": opened()
          }}
        />
        <p
          class="m-0 text-lightSlate-700 flex items-center uppercase text-ellipsis max-w-full text-left"
          classList={{
            "text-md": props.size !== "small",
            "text-xs": props.size === "small"
          }}
        >
          {props.title}
        </p>
      </div>
      <div
        classList={{
          "h-auto": opened(),
          "h-0 overflow-hidden": !opened()
        }}
      >
        {props.children}
      </div>
    </div>
  )
}

export { Collapsable }
