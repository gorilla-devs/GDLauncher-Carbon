import { createSignal, JSX, Show } from "solid-js"

export interface SessionGroupProps {
  title: string
  count: number
  children: JSX.Element
  defaultOpen?: boolean
}

const SessionGroup = (props: SessionGroupProps) => {
  const [isOpen, setIsOpen] = createSignal(props.defaultOpen ?? true)

  return (
    <div class="mb-1.5">
      {/* Group header */}
      <div
        class="flex items-center justify-between px-3 py-1.5 cursor-pointer hover:bg-darkSlate-700/50 rounded-md transition-colors duration-150"
        onClick={() => setIsOpen(!isOpen())}
      >
        <div class="flex items-center gap-1.5">
          <div
            class={`i-hugeicons:arrow-down-01 h-3 w-3 text-lightSlate-600 transition-transform duration-200 ease-out ${!isOpen() ? "-rotate-90" : ""}`}
          />
          <span class="text-lightSlate-500 text-xs font-medium uppercase tracking-wide">
            {props.title}
          </span>
        </div>
        <span class="text-lightSlate-700 text-xs bg-darkSlate-700 px-1.5 py-0.5 rounded-full min-w-[18px] text-center">
          {props.count}
        </span>
      </div>

      {/* Content */}
      <Show when={isOpen()}>
        <div class="mt-0.5 space-y-0.5">{props.children}</div>
      </Show>
    </div>
  )
}

export default SessionGroup
