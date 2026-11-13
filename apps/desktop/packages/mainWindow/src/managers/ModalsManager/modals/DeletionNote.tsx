import { JSX, Show } from "solid-js"
import { DeletionTarget } from "@/utils/deletionTarget"

const DOTS = {
  green: "bg-green-400",
  red: "bg-red-400",
  yellow: "bg-yellow-400"
}

/** The box under a delete question that says what the delete does. */
export function DeletionNoteBox(props: {
  children: JSX.Element
  testId?: string
}) {
  return (
    <div
      class="bg-darkSlate-900 text-lightSlate-300 flex flex-col gap-2 rounded-lg p-3 text-sm"
      data-testid={props.testId}
    >
      {props.children}
    </div>
  )
}

/** One statement in the box, with a dot in the colour of its consequence. */
export function DeletionNoteLine(props: {
  tone: keyof typeof DOTS
  strong?: boolean
  testId?: string
  children: JSX.Element
}) {
  return (
    <div class="flex items-start gap-2" data-testid={props.testId}>
      <div class={`mt-1.5 h-2 w-2 shrink-0 rounded-full ${DOTS[props.tone]}`} />
      <span class={props.strong ? "text-lightSlate-50 font-bold" : ""}>
        {props.children}
      </span>
    </div>
  )
}

/**
 * Where what is deleted goes: the Recycle Bin or nowhere. Nothing is said
 * until the setting is known.
 */
export function DeletionTargetLine(props: {
  target: DeletionTarget
  recycleBin: JSX.Element
  permanent: JSX.Element
}) {
  return (
    <>
      <Show when={props.target === "recycleBin"}>
        <DeletionNoteLine tone="green" testId="deletion-recycle-bin">
          {props.recycleBin}
        </DeletionNoteLine>
      </Show>
      <Show when={props.target === "permanent"}>
        <DeletionNoteLine tone="red" testId="deletion-permanent">
          {props.permanent}
        </DeletionNoteLine>
      </Show>
    </>
  )
}
