import {
  JSX,
  Show,
  Switch,
  Match,
  For,
  createSignal,
  createUniqueId
} from "solid-js"
import { Button } from "../Button"
import { PRESS_CLASSES_LIGHT } from "../Clickable"

interface Props {
  value: string | number | string[] | undefined
  checked: boolean
  /** Radios that share a name are one group for the keyboard: Tab stops
   *  once, on the checked one, and the arrow keys move the choice. */
  name?: string
  /** What else to say about the choice, beyond its label. */
  "aria-describedby"?: string
  onChange?: OnChange
  buttonStyle?: ButtonStyle
  allowDeselect?: boolean
  children?: JSX.Element
}

type OnChange = (_value: string | number | string[] | undefined) => void

interface Option {
  value: string | number | string[] | undefined
  label: JSX.Element
}

type ButtonStyle = "standard" | "button"

interface GroupProps {
  onChange?: OnChange
  value?: string | number | string[] | undefined
  buttonStyle?: ButtonStyle
  options: Option[]
}

/** The radio's dot in a ring: filled when `checked`, an empty ring when
 *  not. For controls that act as a radio without being one. */
export const RadioIndicator = (props: {
  checked: boolean
  hovered?: boolean
  class?: string
}) => (
  <div
    // One ring colour at a time: with both classes the later rule in the
    // stylesheet wins, whichever the state.
    class={`flex justify-center items-center box-border transition-colors w-5 h-5 min-w-5 min-h-5 rounded-full shrink-0 ${
      props.checked
        ? "bg-primary-500"
        : `bg-darkSlate-800 ring-1 ring-inset ${
            props.hovered ? "ring-darkSlate-300" : "ring-darkSlate-500"
          }`
    } ${props.class ?? ""}`}
  >
    <div
      class={`w-2 h-2 rounded-full bg-lightSlate-100 transition-transform ${
        props.checked ? "scale-100" : "scale-0"
      }`}
    />
  </div>
)

const Radio = (props: Props) => {
  const [isHovered, setIsHovered] = createSignal(false) // Track hover state
  let input: HTMLInputElement | undefined

  // Determine base and conditional classes based on buttonStyle
  const baseClasses = `relative flex gap-2 items-center w-full rounded-md px-1.5 py-1 ${PRESS_CLASSES_LIGHT}`

  return (
    <label
      // From the radio only, not a button in the label (a known-issue icon),
      // which has its own. Inset: lists of radios scroll, and their edge
      // clips.
      class={`${baseClasses} cursor-pointer has-[input:focus-visible]:outline has-[input:focus-visible]:outline-2 has-[input:focus-visible]:-outline-offset-2 has-[input:focus-visible]:outline-primary-500`}
      classList={{
        "hover:bg-darkSlate-700": !props.checked,
        "bg-darkSlate-600": props.checked
      }}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      onClick={(e) => {
        // The keyboard checks the radio itself; its change says so below.
        if (e.target === input) return
        // A press on the label: picked (or let go) here, not by the
        // label's own click on the radio, and focus goes to the radio as
        // on a native one, so the arrow keys go on from there.
        e.preventDefault()
        input?.focus({ preventScroll: true })
        if (props.allowDeselect || !props.checked) {
          props.onChange?.(props.value)
        }
      }}
    >
      {/* The real radio, for the keyboard and screen readers. It covers the
          row unseen, so focusing it scrolls the whole row into view, and
          lets the pointer through to the label. */}
      <input
        ref={input}
        type="radio"
        class="pointer-events-none absolute inset-0 m-0 h-full w-full opacity-0"
        name={props.name}
        aria-describedby={props["aria-describedby"]}
        value={props.value?.toString() ?? ""}
        checked={props.checked}
        onChange={() => props.onChange?.(props.value)}
      />
      <Show when={props.buttonStyle === "button"}>
        <Button type={props.checked ? "primary" : "secondary"}>
          {props.children}
        </Button>
      </Show>
      <Show when={props.buttonStyle !== "button"}>
        <RadioIndicator checked={props.checked} hovered={isHovered()} />
        <Show when={props.children}>
          <span class="flex-1">{props.children}</span>
        </Show>
      </Show>
    </label>
  )
}

const Group = (props: GroupProps) => {
  const name = createUniqueId()

  return (
    <Switch>
      <Match when={props.buttonStyle === "button"}>
        <div class="flex bg-darkSlate-900 w-max">
          <For each={props.options}>
            {(option) => (
              <Radio
                name={name}
                value={option.value}
                checked={props.value === option.value}
                onChange={props.onChange}
                buttonStyle={props.buttonStyle}
              >
                {option.label}
              </Radio>
            )}
          </For>
        </div>
      </Match>
      <Match when={props.buttonStyle !== "button"}>
        <For each={props.options}>
          {(option) => (
            <Radio
              name={name}
              value={option.value}
              checked={props.value === option.value}
              onChange={props.onChange}
              buttonStyle={props.buttonStyle}
            >
              {option.label}
            </Radio>
          )}
        </For>
      </Match>
    </Switch>
  )
}

Radio.group = Group

export { Radio }
