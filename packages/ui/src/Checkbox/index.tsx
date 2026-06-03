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
  children?: JSX.Element
}

function Checkbox(props: Props) {
  const [checked, setChecked] = createSignal(props.checked || false)

  createEffect(() => {
    setChecked(props.checked || false)
  })

  return (
    <div
      class={`flex items-center gap-2 rounded-md ${PRESS_CLASSES}`}
      classList={{
        "cursor-pointer": !props.disabled,
        "cursor-not-allowed": props.disabled
      }}
      onPointerDown={(e) => {
        if (props.disabled) return
        // Capture so pointerup lands here even if active:scale-95 shrinks
        // the bounding box out from under a cursor that started near the edge.
        e.currentTarget.setPointerCapture(e.pointerId)
      }}
      onPointerUp={(e) => {
        if (props.disabled) return
        if (!e.currentTarget.hasPointerCapture(e.pointerId)) return
        // Don't bounds-check the pointerup position: getBoundingClientRect
        // returns the active:scale-95 *transformed* rect, which excludes
        // exactly the edge pixels we're trying to fix. Pointer capture
        // already guarantees pointerup landed on the captured element.
        e.currentTarget.releasePointerCapture(e.pointerId)
        const check = !checked()
        setChecked(check)
        props.onChange?.(check)
      }}
    >
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
    </div>
  )
}

export { Checkbox }
