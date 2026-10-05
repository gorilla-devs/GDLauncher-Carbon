import {
  JSX,
  Show,
  Switch,
  Match,
  For,
  createUniqueId,
  createSignal
} from "solid-js"
import { Button } from "../Button"
import { PRESS_CLASSES_LIGHT } from "../Clickable"

interface Props {
  value: string | number | string[] | undefined
  checked: boolean
  onChange?: OnChange
  buttonStyle?: ButtonStyle
  allowDeselect?: boolean
  /** Shared by choices in one logical group; distinct panel copies use distinct names. */
  name?: string
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
  name?: string
}

const Radio = (props: Props) => {
  const [isHovered, setIsHovered] = createSignal(false) // Track hover state

  const id = `radio-${createUniqueId()}`

  // Determine base and conditional classes based on buttonStyle
  const baseClasses = `relative flex gap-2 items-center w-full rounded-md px-1.5 py-1 ${PRESS_CLASSES_LIGHT}`
  const indicatorBaseClasses = "w-5 h-5 min-w-5 min-h-5 rounded-full shrink-0"

  return (
    <>
      <input
        type="radio"
        class={props.buttonStyle === "button" ? "hidden" : "peer sr-only"}
        name={props.name ?? id}
        value={
          typeof props.value === "string" || typeof props.value === "number"
            ? props.value
            : undefined
        }
        checked={props.checked}
        id={id}
        onChange={() => {
          if (props.buttonStyle !== "button") props.onChange?.(props.value)
        }}
        onClick={() => {
          if (
            props.buttonStyle !== "button" &&
            props.allowDeselect &&
            props.checked
          ) {
            props.onChange?.(props.value)
          }
        }}
        onKeyDown={(event) => {
          if (
            props.buttonStyle !== "button" &&
            props.allowDeselect &&
            props.checked &&
            event.key === " "
          ) {
            event.preventDefault()
            props.onChange?.(props.value)
          }
        }}
      />
      <label
        for={id}
        class={`${baseClasses} cursor-pointer peer-focus-visible:outline peer-focus-visible:outline-2 peer-focus-visible:outline-primary-500`}
        classList={{
          "hover:bg-darkSlate-700": !props.checked,
          "bg-darkSlate-600": props.checked
        }}
        onMouseEnter={() => setIsHovered(true)}
        onMouseLeave={() => setIsHovered(false)}
        onClick={(event) => {
          if (
            props.buttonStyle === "button" ||
            (props.allowDeselect && props.checked)
          ) {
            // Prevent the label's forwarded native click from calling twice.
            event.preventDefault()
            if (props.allowDeselect || !props.checked)
              props.onChange?.(props.value)
          }
        }}
      >
        <Show when={props.buttonStyle === "button"}>
          <Button type={props.checked ? "primary" : "secondary"}>
            {props.children}
          </Button>
        </Show>
        <Show when={props.buttonStyle !== "button"}>
          <div
            class={`flex justify-center items-center box-border transition-colors ${indicatorBaseClasses} ${
              props.checked
                ? "bg-primary-500"
                : "bg-darkSlate-800 ring-1 ring-inset ring-darkSlate-500"
            } ${isHovered() && !props.checked ? "ring-darkSlate-300" : ""}`}
          >
            <div
              class={`w-2 h-2 rounded-full bg-lightSlate-100 transition-transform ${
                props.checked ? "scale-100" : "scale-0"
              }`}
            />
          </div>
          <Show when={props.children}>
            <span class="flex-1">{props.children}</span>
          </Show>
        </Show>
      </label>
    </>
  )
}

const Group = (props: GroupProps) => {
  const name = `radio-group-${createUniqueId()}`
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
