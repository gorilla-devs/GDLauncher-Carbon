import {
  JSX,
  Show,
  Switch,
  Match,
  For,
  splitProps,
  createSignal
} from "solid-js"
import { Button } from "../Button"
import { PRESS_CLASSES_LIGHT } from "../Clickable"

interface Props {
  value: string | number | string[] | undefined
  checked: boolean
  onChange?: OnChange
  buttonStyle?: ButtonStyle
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

let nextId = 1

const Radio = (props: Props) => {
  const [local, otherProps] = splitProps(props, ["buttonStyle", "onChange"])
  const [isHovered, setIsHovered] = createSignal(false) // Track hover state

  const id = `radio-${nextId++}`

  // Determine base and conditional classes based on buttonStyle
  const baseClasses = `relative flex gap-4 items-center w-full rounded-lg px-4 py-3 ${PRESS_CLASSES_LIGHT}`
  const indicatorBaseClasses = "w-5 h-5 min-w-5 min-h-5 rounded-full shrink-0"

  return (
    <>
      <input
        type="radio"
        class="hidden"
        {...otherProps}
        checked={props.checked}
        id={id}
      />
      <label
        for={id}
        class={`${baseClasses} cursor-pointer`}
        classList={{
          "hover:bg-darkSlate-700": !props.checked,
          "bg-darkSlate-600": props.checked
        }}
        onMouseEnter={() => setIsHovered(true)}
        onMouseLeave={() => setIsHovered(false)}
        onClick={() => {
          if (!props.checked) {
            local?.onChange?.(props.value)
          }
        }}
      >
        <Show when={local?.buttonStyle === "button"}>
          <Button type={props.checked ? "primary" : "secondary"}>
            {props.children}
          </Button>
        </Show>
        <Show when={local?.buttonStyle !== "button"}>
          <div
            class={`flex justify-center items-center box-border transition-colors ${indicatorBaseClasses} ${
              props.checked
                ? "bg-primary-500"
                : "bg-darkSlate-500 border border-darkSlate-300"
            } ${isHovered() && !props.checked ? "border-darkSlate-200" : ""}`}
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
  return (
    <Switch>
      <Match when={props.buttonStyle === "button"}>
        <div class="flex bg-darkSlate-900 w-max">
          <For each={props.options}>
            {(option) => (
              <Radio
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
