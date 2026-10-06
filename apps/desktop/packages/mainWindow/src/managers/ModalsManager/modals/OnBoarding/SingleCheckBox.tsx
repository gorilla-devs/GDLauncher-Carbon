import { Checkbox } from "@gd/ui"
import { For, Setter, Show } from "solid-js"
import { instances } from "./SingleEntity"

interface Props {
  title?: string
  filename?: string
  selectionId?: string
  invalidReason?: string
  /** What will be left out of this import; the row can still be selected. */
  warnings?: string[]
  setList?: Setter<string[]>
  setInstance?: (_instance: string | undefined) => void
  isSingleInstance?: boolean
}

const SingleCheckBox = (props: Props) => {
  // The session and backend index identify equal filenames across source roots.
  const uniqueId = () => props.selectionId || props.filename || props.title

  const isInvalid = () => props.invalidReason !== undefined

  const isSelected = () =>
    !isInvalid() && instances().some((instance) => instance === uniqueId())

  const handleChange = (checked: boolean) => {
    if (isInvalid()) return

    if (!checked) {
      if (props.setList) {
        props.setList((prev) => prev.filter((e) => e !== uniqueId()))
      }
    } else {
      if (props.setList) {
        const id = uniqueId()
        if (id) {
          props.setList((prev) => (prev.includes(id) ? prev : [...prev, id]))
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
      onClick={() => handleChange(!isSelected())}
    >
      <Show
        when={!isInvalid()}
        fallback={
          <div class="i-hugeicons:alert-02 text-yellow-500 shrink-0 text-xl" />
        }
      >
        <div onClick={(event) => event.stopPropagation()}>
          <Checkbox
            checked={isSelected()}
            ariaLabel={props.title ?? props.filename}
            onChange={handleChange}
          />
        </div>
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
          <span class="break-words text-xs text-yellow-500/80">
            {props.invalidReason}
          </span>
        </Show>
        <Show when={!isInvalid() && props.filename}>
          <span class="text-lightSlate-500 flex items-center gap-1 truncate text-xs">
            <div class="i-hugeicons:folder-01 shrink-0 text-xs" />
            {props.filename}
          </span>
        </Show>
        <Show when={!isInvalid()}>
          <For each={props.warnings}>
            {(warning) => (
              <span class="flex items-start gap-1 break-words text-xs text-yellow-500/80">
                <div class="i-hugeicons:alert-02 mt-0.5 shrink-0 text-xs" />
                {warning}
              </span>
            )}
          </For>
        </Show>
      </div>
    </div>
  )
}

export default SingleCheckBox
