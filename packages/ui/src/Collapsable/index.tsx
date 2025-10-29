import { JSX, createSignal } from "solid-js"
import { AnimatedIcon } from "../AnimatedIcon"

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
        class="max-w-full h-8 flex gap-2 items-center cursor-pointer"
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
        <AnimatedIcon
          icon="i-hugeicons:arrow-down-01"
          size="min-w-4 min-h-4"
          class="transition ease-in-out text-lightSlate-700"
          classList={{
            "-rotate-180": !opened()
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
