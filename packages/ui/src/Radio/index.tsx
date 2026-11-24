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
import { withStates } from "../hooks"

interface Props {
  value: string | number | string[] | undefined
  checked: boolean
  /** Radios that share a name are one group for the keyboard: Tab stops
   *  once, on the checked one, and the arrow keys move the choice. */
  name?: string
  /** What else to say about the choice, beyond its label. */
  "aria-describedby"?: string
  disabled?: boolean
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

/** Sets one radio input's native checked state back to its `checked` prop. */
const restoreChecked = new WeakMap<HTMLInputElement, () => void>()

/**
 * The browser checks a radio the keyboard picks, and unchecks the rest of
 * its group, before `onChange` runs, and picking a radio that is already
 * checked fires no change. Once the handler has run, set every radio of the
 * group back to its `checked` prop: if the parent did not apply the choice
 * (a refused or failed save), picking it again then fires change again, and
 * the browser reports the option that is drawn as selected. Solid sets the
 * new state when the parent's value changes.
 */
const restoreGroup = (input: HTMLInputElement) =>
  queueMicrotask(() => {
    const root = input.getRootNode() as ParentNode
    root
      .querySelectorAll<HTMLInputElement>('input[type="radio"]')
      .forEach((radio) => {
        if (radio.name === input.name) restoreChecked.get(radio)?.()
      })
  })

interface GroupProps {
  onChange?: OnChange
  value?: string | number | string[] | undefined
  buttonStyle?: ButtonStyle
  options: Option[]
  /** Shared by the group's radios; distinct panel copies use distinct names. */
  name?: string
}

/** The radio's dot in a ring: filled when `checked`, an empty ring when
 *  not. For controls that act as a radio without being one. */
export const RadioIndicator = (props: {
  checked: boolean
  hovered?: boolean
  class?: string
}) => (
  <div
    data-gdl="radio.dot"
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
      data-gdl={withStates("radio", { checked: props.checked })}
      // From the radio only, not a button in the label (a known-issue icon),
      // which has its own. Inset: lists of radios scroll, and their edge
      // clips.
      class={`${baseClasses} cursor-pointer has-[input:focus-visible]:outline has-[input:focus-visible]:outline-2 has-[input:focus-visible]:-outline-offset-2 has-[input:focus-visible]:outline-primary-500`}
      classList={{
        "hover:bg-darkSlate-700": !props.checked && !props.disabled,
        "opacity-50 !cursor-not-allowed": props.disabled,
        "bg-darkSlate-600": props.checked
      }}
      onMouseEnter={() => {
        if (!props.disabled) setIsHovered(true)
      }}
      onMouseLeave={() => setIsHovered(false)}
      onClick={(e) => {
        // The keyboard checks the radio itself; its change says so below.
        if (e.target === input) return
        if (props.disabled) {
          e.preventDefault()
          return
        }
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
        ref={(el) => {
          input = el
          restoreChecked.set(el, () => {
            el.checked = props.checked
          })
        }}
        type="radio"
        class="pointer-events-none absolute inset-0 m-0 h-full w-full opacity-0"
        name={props.name}
        disabled={props.disabled}
        aria-describedby={props["aria-describedby"]}
        value={props.value?.toString() ?? ""}
        checked={props.checked}
        onChange={(e) => {
          restoreGroup(e.currentTarget)
          if (props.disabled) return
          props.onChange?.(props.value)
        }}
        // A click on the radio itself, as assistive tech activates it (the
        // pointer lands on the label): on the checked one it lets go where
        // that is allowed. The browser fires no change there.
        onClick={() => {
          if (props.disabled) return
          if (props.allowDeselect && props.checked) {
            props.onChange?.(props.value)
          }
        }}
        // Space on the checked radio lets it go where that is allowed; the
        // browser does nothing there by itself.
        onKeyDown={(e) => {
          if (props.disabled || e.key !== " ") return
          if (props.allowDeselect && props.checked) {
            e.preventDefault()
            props.onChange?.(props.value)
          }
        }}
      />
      <Show when={props.buttonStyle === "button"}>
        <Button
          disabled={props.disabled}
          type={props.checked ? "primary" : "secondary"}
        >
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
                name={props.name ?? name}
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
              name={props.name ?? name}
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
