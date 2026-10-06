import { createEffect, createSignal, Show, JSX } from "solid-js"
import { cva, type VariantProps } from "class-variance-authority"
import { PRESS_CLASSES } from "../Clickable"

const checkboxStyles = cva(
  "box-border flex h-5 min-h-5 w-5 min-w-5 items-center justify-center rounded-md transition-all duration-200 ease-spring",
  {
    variants: {
      checked: {
        true: "bg-primary-500",
        false: "bg-darkSlate-800 ring-1 ring-inset ring-darkSlate-500"
      },
      disabled: {
        true: "bg-darkSlate-900",
        false: ""
      },
      indeterminate: {
        true: "bg-light-300/20",
        false: ""
      },
      hover: {
        true: "hover:ring-darkSlate-300 hover:ring-1 hover:ring-inset",
        false: ""
      }
    },
    defaultVariants: {
      checked: false,
      disabled: false,
      indeterminate: false,
      hover: true
    }
  }
)

interface Props extends VariantProps<typeof checkboxStyles> {
  checked?: boolean
  indeterminate?: boolean
  disabled?: boolean
  onChange?: (_checked: boolean) => void
  ariaLabel?: string
  children?: JSX.Element
}

function Checkbox(props: Props) {
  const [checked, setChecked] = createSignal(props.checked || false)
  let input: HTMLInputElement | undefined

  createEffect(() => {
    checked()
    if (input) input.indeterminate = !!props.indeterminate
  })

  createEffect(() => {
    setChecked(props.checked || false)
  })

  return (
    <label
      class={`relative flex items-center gap-2 rounded-md focus-within:ring-2 focus-within:ring-primary-400 ${PRESS_CLASSES}`}
      classList={{
        "cursor-pointer": !props.disabled,
        "cursor-not-allowed": props.disabled
      }}
      onPointerDown={(event) => {
        if (props.disabled || !props.onChange) return
        event.currentTarget.setPointerCapture(event.pointerId)
      }}
      onPointerUp={(event) => {
        if (event.currentTarget.hasPointerCapture(event.pointerId)) {
          event.currentTarget.releasePointerCapture(event.pointerId)
        }
      }}
    >
      <input
        ref={input}
        type="checkbox"
        class="sr-only"
        aria-label={props.ariaLabel}
        aria-hidden={!props.onChange ? true : undefined}
        tabIndex={props.onChange ? 0 : -1}
        checked={checked()}
        disabled={!!props.disabled || !props.onChange}
        onChange={(event) => {
          if (props.disabled) return
          const value = event.currentTarget.checked
          setChecked(value)
          props.onChange?.(value)
        }}
        onKeyDown={(event) => {
          if (event.key !== "Enter") return
          event.preventDefault()
          if (!props.disabled && !event.repeat) event.currentTarget.click()
        }}
      />
      <div
        class={checkboxStyles({
          checked: checked(),
          disabled: props.disabled,
          indeterminate: props.indeterminate,
          hover: !props.disabled
        })}
      >
        <Show when={!checked() && props.indeterminate}>
          <div class="bg-primary-500 h-3 min-h-3 w-3 min-w-3 rounded-sm" />
        </Show>
        <Show when={!props.indeterminate || checked()}>
          <div
            class="i-hugeicons:tick-02 h-4 w-4 transition-transform duration-200 ease-spring"
            classList={{
              "scale-100": checked(),
              "scale-0": !checked(),
              "text-lightSlate-50": !props.disabled,
              "text-lightSlate-700": props.disabled
            }}
          />
        </Show>
      </div>
      <Show when={props.children}>{props.children}</Show>
    </label>
  )
}

export { Checkbox }
