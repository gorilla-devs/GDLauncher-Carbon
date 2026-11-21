import {
  children,
  createEffect,
  createSignal,
  createUniqueId,
  Show,
  JSX
} from "solid-js"
import { cva, type VariantProps } from "class-variance-authority"
import { PRESS_CLASSES } from "../Clickable"
import { withStates } from "../hooks"

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
  /** Without it, the checkbox is only drawn, for a parent that handles the
   *  click (a tile, a row): it then takes no focus of its own. */
  onChange?: (_checked: boolean) => void
  /** Its name when its label isn't among its children. */
  "aria-label"?: string
  children?: JSX.Element
}

function Checkbox(props: Props) {
  const [checked, setChecked] = createSignal(props.checked || false)
  const label = children(() => props.children)
  const labelId = createUniqueId()
  const [input, setInput] = createSignal<HTMLInputElement>()

  createEffect(() => {
    setChecked(props.checked || false)
  })

  // A property only: screen readers then say "partly checked".
  createEffect(() => {
    const el = input()
    if (el) el.indeterminate = !checked() && !!props.indeterminate
  })

  return (
    <div
      data-gdl={withStates("checkbox", {
        checked: checked(),
        // The dash it draws when it's not ticked.
        indeterminate: !checked() && props.indeterminate,
        disabled: props.disabled
      })}
      class={`relative flex items-center gap-2 rounded-md ${PRESS_CLASSES}`}
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
      {/* The real checkbox, for the keyboard and screen readers: Space
          toggles it. It covers the row unseen, so focusing it scrolls the
          whole row into view, and lets the pointer through to the press
          handling above. */}
      <Show when={props.onChange}>
        <input
          ref={setInput}
          type="checkbox"
          class="peer pointer-events-none absolute inset-0 m-0 h-full w-full opacity-0"
          checked={checked()}
          disabled={props.disabled}
          aria-labelledby={label() ? labelId : undefined}
          aria-label={props["aria-label"]}
          onChange={(e) => {
            const check = e.currentTarget.checked
            setChecked(check)
            props.onChange?.(check)
          }}
        />
      </Show>
      <div
        data-gdl="checkbox.box"
        class={`${checkboxStyles({
          checked: checked(),
          disabled: props.disabled,
          indeterminate: props.indeterminate,
          hover: !props.disabled
        })} peer-focus-visible:outline peer-focus-visible:outline-2 peer-focus-visible:outline-offset-2 peer-focus-visible:outline-primary-500`}
      >
        <Show when={!checked() && props.indeterminate}>
          <div
            data-gdl="checkbox.dash"
            class="bg-primary-500 h-3 min-h-3 w-3 min-w-3 rounded-sm"
          />
        </Show>
        <Show when={!props.indeterminate || checked()}>
          <div
            data-gdl="checkbox.mark"
            class="i-hugeicons:tick-02 h-4 w-4 transition-transform duration-200 ease-spring"
            classList={{
              "scale-100": checked(),
              "scale-0": !checked(),
              // On the accent fill: `--gdl-on-primary` (style.css).
              "gdl-on-primary": !props.disabled,
              "text-lightSlate-700": props.disabled
            }}
          />
        </Show>
      </div>
      <Show when={label()}>
        {/* No box of its own: the label lays out as before. */}
        <span id={labelId} class="contents">
          {label()}
        </span>
      </Show>
    </div>
  )
}

export { Checkbox }
