import { createSignal, onCleanup, type Accessor, type JSX } from "solid-js"

/** How long a press must last to confirm, unless a control sets its own. */
export const HOLD_TO_CONFIRM_MS = 1200
/** A press shorter than this is a click, which only shows the hint. */
const TAP_MS = 300
/** How long the hint stays after a click. */
const HINT_MS = 2000

export interface HoldToConfirmOptions {
  /** Runs once the press has lasted the whole duration. */
  onConfirm: () => void
  /** Milliseconds the press must last. Read when a press starts. */
  duration?: number | Accessor<number | undefined>
  /** While true a press does nothing. Read when a press starts. */
  disabled?: Accessor<boolean>
}

export interface HoldToConfirm {
  /** True from the start of a press until it confirms or is let go. */
  holding: Accessor<boolean>
  /** True for a moment after a click, for the control to say "hold". */
  hinted: Accessor<boolean>
  /** The duration of the press now held, or of the next one. */
  duration: Accessor<number>
  /**
   * True, once, for the click that ends a press which confirmed. Letting go
   * of the mouse or the key is still a click on the control, and by then the
   * control may be something else (Stop became Play): its own click handler
   * asks this first and does nothing when it is true.
   */
  claimsClick: () => boolean
  /** Spread on the element that is pressed. A click alone does nothing. */
  handlers: {
    onPointerDown: (event: PointerEvent) => void
    onPointerUp: () => void
    onPointerCancel: () => void
    onKeyDown: (event: KeyboardEvent) => void
    onKeyUp: (event: KeyboardEvent) => void
    onBlur: () => void
    onClick: (event: MouseEvent) => void
  }
}

const isPressKey = (event: KeyboardEvent) =>
  event.key === " " || event.key === "Enter"

/**
 * Hold to confirm: an action that runs only once its control has been
 * pressed for a moment, with the mouse, a finger, Space or Enter. Letting go
 * early cancels, so a stray click can never trigger it; a click only raises
 * `hinted`, for the control to say that it must be held.
 *
 * It is the logic alone. Spread `handlers` on the pressed element, and show
 * `holding` with [`HoldFill`] or [`HoldRing`], which run for `duration`.
 * `Button` has it built in as its `holdToConfirm` prop.
 */
export function createHoldToConfirm(
  options: HoldToConfirmOptions
): HoldToConfirm {
  const [holding, setHolding] = createSignal(false)
  const [hinted, setHinted] = createSignal(false)
  const configured = () =>
    (typeof options.duration === "function"
      ? options.duration()
      : options.duration) ?? HOLD_TO_CONFIRM_MS
  const [duration, setDuration] = createSignal(configured())
  let confirmTimer: ReturnType<typeof setTimeout> | undefined
  let hintTimer: ReturnType<typeof setTimeout> | undefined
  let startedAt = 0
  // The press that is still down has confirmed.
  let confirmed = false
  // Until the mouse button or the key really comes up, wherever that is:
  // the control may have changed under it, lost focus, or been left. The
  // click of that release comes right after it, in the same turn; any later
  // click is a new one.
  const pressEnded = () => {
    window.removeEventListener("pointerup", pressEnded, true)
    window.removeEventListener("keyup", pressEnded, true)
    setTimeout(() => (confirmed = false), 0)
  }
  const confirm = () => {
    confirmed = true
    window.addEventListener("pointerup", pressEnded, true)
    window.addEventListener("keyup", pressEnded, true)
    options.onConfirm()
  }

  const start = () => {
    if (holding() || options.disabled?.()) return
    startedAt = Date.now()
    clearTimeout(hintTimer)
    setHinted(false)
    setDuration(configured())
    setHolding(true)
    confirmTimer = setTimeout(() => {
      confirmTimer = undefined
      setHolding(false)
      confirm()
    }, duration())
  }

  const release = () => {
    if (!holding()) return
    clearTimeout(confirmTimer)
    confirmTimer = undefined
    setHolding(false)
    if (Date.now() - startedAt < TAP_MS) {
      setHinted(true)
      clearTimeout(hintTimer)
      hintTimer = setTimeout(() => setHinted(false), HINT_MS)
    }
  }

  onCleanup(() => {
    clearTimeout(confirmTimer)
    clearTimeout(hintTimer)
    window.removeEventListener("pointerup", pressEnded, true)
    window.removeEventListener("keyup", pressEnded, true)
  })

  return {
    holding,
    hinted,
    duration,
    claimsClick: () => {
      const claimed = confirmed
      confirmed = false
      return claimed
    },
    handlers: {
      onPointerDown: (event) => {
        // The main button only; and never a drag or a click on what is
        // behind the control.
        if (event.button !== 0) return
        event.stopPropagation()
        const target = event.currentTarget
        if (target instanceof Element && target.setPointerCapture) {
          try {
            target.setPointerCapture(event.pointerId)
          } catch {
            // Not a live pointer; the press still counts.
          }
        }
        start()
      },
      onPointerUp: release,
      onPointerCancel: release,
      onKeyDown: (event) => {
        if (!isPressKey(event)) return
        event.preventDefault()
        event.stopPropagation()
        if (!event.repeat) start()
      },
      onKeyUp: (event) => {
        if (isPressKey(event)) release()
      },
      onBlur: release,
      onClick: (event) => {
        confirmed = false
        event.preventDefault()
        event.stopPropagation()
      }
    }
  }
}

const transition = (
  hold: Pick<HoldToConfirm, "holding" | "duration">,
  property: string
): JSX.CSSProperties => ({
  "transition-property": property,
  // Fills for as long as the press must last; empties at once when let go.
  "transition-timing-function": hold.holding() ? "linear" : "ease-out",
  "transition-duration": hold.holding() ? `${hold.duration()}ms` : "180ms"
})

/**
 * The fill of a held control: a layer that grows from its left edge to its
 * full width while the press lasts. Put it first inside a `relative
 * overflow-hidden` control, and keep the control's content `relative` so it
 * stays above the fill.
 */
export function HoldFill(props: {
  hold: Pick<HoldToConfirm, "holding" | "duration">
  /** The fill's colour; a dark veil by default. */
  class?: string
}) {
  return (
    <div
      aria-hidden="true"
      data-hold-fill=""
      class={`pointer-events-none absolute inset-y-0 left-0 ${props.class ?? "bg-black/40"}`}
      style={{
        width: props.hold.holding() ? "100%" : "0%",
        ...transition(props.hold, "width")
      }}
    />
  )
}

/**
 * The same for a round control, such as an avatar: a ring that closes
 * around it while the press lasts. It covers its `relative` parent.
 */
export function HoldRing(props: {
  hold: Pick<HoldToConfirm, "holding" | "duration">
  /** The ring's colour, as a text colour class; white by default. */
  class?: string
}) {
  return (
    <svg
      aria-hidden="true"
      data-hold-ring=""
      viewBox="0 0 36 36"
      class={`pointer-events-none absolute inset-0 h-full w-full -rotate-90 ${props.class ?? "text-white"}`}
    >
      <circle
        cx="18"
        cy="18"
        r="16.5"
        fill="none"
        stroke="currentColor"
        stroke-width="3"
        stroke-linecap="round"
        pathLength="1"
        stroke-dasharray="1"
        style={{
          "stroke-dashoffset": props.hold.holding() ? "0" : "1",
          ...transition(props.hold, "stroke-dashoffset")
        }}
      />
    </svg>
  )
}
