import { Checkbox } from "@gd/ui"
import { Setter, Show } from "solid-js"
import { instances } from "./SingleEntity"

interface Props {
  title?: string
  filename?: string
  invalidReason?: string
  setList?: Setter<string[]>
  setInstance?: (_instance: string | undefined) => void
  isSingleInstance?: boolean
}

const SingleCheckBox = (props: Props) => {
  // Use filename as unique identifier since instance names can be duplicated
  const uniqueId = () => props.filename || props.title

  const isInvalid = () => props.invalidReason !== undefined

  const isSelected = () =>
    !isInvalid() && instances().some((instance) => instance === uniqueId())

  const handleChange = () => {
    if (isInvalid()) return

    if (isSelected()) {
      if (props.setList) {
        props.setList((prev) => prev.filter((e) => e !== uniqueId()))
      }
    } else {
      if (props.setList) {
        const id = uniqueId()
        if (id) {
          props.setList((prev) => [...prev, id])
        }
      }
    }
  }

  return (
    <div
      class={`flex items-center gap-3 rounded-lg p-3 transition-all ${
        isInvalid()
          ? "bg-darkSlate-800/60 cursor-not-allowed"
          : isSelected()
            ? "bg-primary-500/10 ring-1 ring-primary-500 cursor-pointer"
            : "bg-darkSlate-800 outline outline-1 outline-transparent hover:outline-darkSlate-500 cursor-pointer"
      }`}
      onClick={handleChange}
    >
      <Show
        when={!isInvalid()}
        fallback={
          <div class="i-hugeicons:alert-02 text-yellow-500 shrink-0 text-xl" />
        }
      >
        <Checkbox checked={isSelected()} onChange={handleChange} />
      </Show>
      <div
        class={`flex min-w-0 flex-1 flex-col gap-0.5 ${
          isInvalid() ? "text-lightSlate-500" : ""
        }`}
      >
        <span class="truncate font-medium">
          {props.title ?? props.filename}
        </span>
        <Show when={isInvalid()}>
          <span class="truncate text-xs text-yellow-500/80">
            {props.invalidReason}
          </span>
        </Show>
        <Show when={!isInvalid() && props.filename}>
          <span class="text-lightSlate-500 flex items-center gap-1 truncate text-xs">
            <div class="i-hugeicons:folder-01 shrink-0 text-xs" />
            {props.filename}
          </span>
        </Show>
      </div>
    </div>
  )
}

export default SingleCheckBox
