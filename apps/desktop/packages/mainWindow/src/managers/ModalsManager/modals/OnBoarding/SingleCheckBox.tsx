import { Checkbox } from "@gd/ui"
import { Setter } from "solid-js"
import { instances } from "./SingleEntity"

interface Props {
  title?: string
  filename?: string
  setList?: Setter<string[]>
  setInstance?: (_instance: string | undefined) => void
  isSingleInstance?: boolean
}

const SingleCheckBox = (props: Props) => {
  // Use filename as unique identifier since instance names can be duplicated
  const uniqueId = () => props.filename || props.title

  const isSelected = () =>
    instances().some((instance) => instance === uniqueId())

  const handleChange = () => {
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
      class={`flex cursor-pointer items-center gap-3 rounded-lg p-3 transition-all ${
        isSelected()
          ? "bg-primary-500/10 ring-1 ring-primary-500"
          : "bg-darkSlate-800 outline outline-1 outline-transparent hover:outline-darkSlate-500"
      }`}
      onClick={handleChange}
    >
      <Checkbox checked={isSelected()} onChange={handleChange} />
      <div class="flex min-w-0 flex-1 flex-col gap-0.5">
        <span class="truncate font-medium">{props.title}</span>
        {props.filename && (
          <span class="text-lightSlate-500 flex items-center gap-1 truncate text-xs">
            <div class="i-hugeicons:folder-01 shrink-0 text-xs" />
            {props.filename}
          </span>
        )}
      </div>
    </div>
  )
}

export default SingleCheckBox
