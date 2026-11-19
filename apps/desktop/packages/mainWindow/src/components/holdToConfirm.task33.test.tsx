import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { createRoot, createSignal } from "solid-js"
import { render } from "solid-js/web"
import {
  createHoldToConfirm,
  HoldFill,
  HoldRing,
  HOLD_TO_CONFIRM_MS,
  type HoldToConfirm
} from "../../../../../../packages/ui/src/HoldToConfirm"
import { Button } from "../../../../../../packages/ui/src/Button"

let dispose: (() => void) | undefined
beforeEach(() => {
  vi.useFakeTimers()
})
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
  vi.useRealTimers()
})

const hold = (options: Parameters<typeof createHoldToConfirm>[0]) => {
  let made!: HoldToConfirm
  dispose = createRoot((stop) => {
    made = createHoldToConfirm(options)
    return stop
  })
  return made
}
const press = (target: HoldToConfirm, button = 0) =>
  target.handlers.onPointerDown(
    new PointerEvent("pointerdown", { button, bubbles: true })
  )

it("confirms only once the press has lasted the whole duration", () => {
  const confirm = vi.fn()
  const h = hold({ onConfirm: confirm })
  expect(h.duration()).toBe(HOLD_TO_CONFIRM_MS)
  press(h)
  expect(h.holding()).toBe(true)
  vi.advanceTimersByTime(HOLD_TO_CONFIRM_MS - 1)
  expect(confirm).not.toHaveBeenCalled()
  vi.advanceTimersByTime(1)
  expect(confirm).toHaveBeenCalledTimes(1)
  expect(h.holding()).toBe(false)
  // Letting go afterwards is not a second anything.
  h.handlers.onPointerUp()
  vi.advanceTimersByTime(5000)
  expect(confirm).toHaveBeenCalledTimes(1)
  expect(h.hinted()).toBe(false)
})

it("cancels when let go early, and hints only after a click", () => {
  const confirm = vi.fn()
  const h = hold({ onConfirm: confirm })
  // A press given up half way: nothing, and no hint.
  press(h)
  vi.advanceTimersByTime(600)
  h.handlers.onPointerUp()
  expect(h.holding()).toBe(false)
  expect(h.hinted()).toBe(false)
  // A click: the hint, for a moment.
  press(h)
  vi.advanceTimersByTime(80)
  h.handlers.onPointerUp()
  expect(h.hinted()).toBe(true)
  vi.advanceTimersByTime(1999)
  expect(h.hinted()).toBe(true)
  vi.advanceTimersByTime(1)
  expect(h.hinted()).toBe(false)
  vi.advanceTimersByTime(5000)
  expect(confirm).not.toHaveBeenCalled()
  // Losing the pointer or the focus lets go as well.
  press(h)
  h.handlers.onPointerCancel()
  press(h)
  h.handlers.onBlur()
  vi.advanceTimersByTime(5000)
  expect(confirm).not.toHaveBeenCalled()
})

it("is held with Space or Enter as with the mouse, and ignores other buttons and keys", () => {
  const confirm = vi.fn()
  const h = hold({ onConfirm: confirm, duration: 500 })
  const key = (type: string, init: KeyboardEventInit) => {
    const event = new KeyboardEvent(type, { cancelable: true, ...init })
    if (type === "keydown") h.handlers.onKeyDown(event)
    else h.handlers.onKeyUp(event)
    return event
  }
  const down = key("keydown", { key: " " })
  expect(down.defaultPrevented).toBe(true)
  expect(h.holding()).toBe(true)
  // The key repeating while held is the same press.
  vi.advanceTimersByTime(300)
  key("keydown", { key: " ", repeat: true })
  vi.advanceTimersByTime(200)
  expect(confirm).toHaveBeenCalledTimes(1)

  key("keydown", { key: "Enter" })
  vi.advanceTimersByTime(400)
  key("keyup", { key: "Enter" })
  vi.advanceTimersByTime(1000)
  expect(confirm).toHaveBeenCalledTimes(1)

  expect(key("keydown", { key: "a" }).defaultPrevented).toBe(false)
  expect(h.holding()).toBe(false)
  press(h, 2)
  expect(h.holding()).toBe(false)
})

it("does nothing while disabled, reads its duration at each press, and never lets a click through", () => {
  const confirm = vi.fn()
  const [disabled, setDisabled] = createSignal(true)
  const [duration, setDuration] = createSignal<number | undefined>(300)
  const h = hold({ onConfirm: confirm, disabled, duration })
  press(h)
  expect(h.holding()).toBe(false)
  setDisabled(false)
  press(h)
  vi.advanceTimersByTime(300)
  expect(confirm).toHaveBeenCalledTimes(1)
  setDuration(undefined)
  press(h)
  expect(h.duration()).toBe(HOLD_TO_CONFIRM_MS)
  vi.advanceTimersByTime(300)
  expect(confirm).toHaveBeenCalledTimes(1)
  h.handlers.onPointerUp()

  const outer = vi.fn()
  const root = document.createElement("div")
  root.addEventListener("click", outer)
  const inner = document.createElement("button")
  inner.addEventListener("click", h.handlers.onClick)
  root.append(inner)
  const click = new MouseEvent("click", { bubbles: true, cancelable: true })
  inner.dispatchEvent(click)
  expect(click.defaultPrevented).toBe(true)
  expect(outer).not.toHaveBeenCalled()
})

it("fills and rings for as long as the press must last, and empties at once", () => {
  const h = hold({ onConfirm: () => {}, duration: 900 })
  const root = document.createElement("div")
  document.body.append(root)
  const unmount = render(
    () => (
      <>
        <HoldFill hold={h} />
        <HoldRing hold={h} />
      </>
    ),
    root
  )
  const fill = root.querySelector<HTMLElement>("[data-hold-fill]")!
  const ring = root.querySelector<SVGCircleElement>("[data-hold-ring] circle")!
  expect(fill.style.width).toBe("0%")
  expect(ring.style.strokeDashoffset).toBe("1")
  press(h)
  expect(fill.style.width).toBe("100%")
  expect(fill.style.transitionDuration).toBe("900ms")
  expect(ring.style.strokeDashoffset).toBe("0")
  expect(ring.style.transitionDuration).toBe("900ms")
  h.handlers.onPointerUp()
  expect(fill.style.width).toBe("0%")
  expect(fill.style.transitionDuration).toBe("180ms")
  unmount()
})

const mountButton = (props: {
  hold?: { onConfirm: () => void; hint?: string; duration?: number }
  onClick?: any
  disabled?: boolean
  loading?: boolean
}) => {
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => (
      <Button
        holdToConfirm={props.hold}
        onClick={props.onClick}
        disabled={props.disabled}
        loading={props.loading}
      >
        Stop
      </Button>
    ),
    root
  )
  return root.querySelector("button")!
}
const pointer = (target: Element, type: string) =>
  target.dispatchEvent(
    new PointerEvent(type, { button: 0, bubbles: true, cancelable: true })
  )

it("a Button with holdToConfirm acts when held, and a click on it only shows the hint", () => {
  const confirm = vi.fn()
  const click = vi.fn()
  const button = mountButton({
    hold: { onConfirm: confirm, hint: "Hold to stop" },
    onClick: click
  })
  expect(button.textContent).toBe("Stop")
  expect(button.querySelector("[data-hold-fill]")).not.toBeNull()

  // A click: the pointer goes down and up, then the click itself.
  pointer(button, "pointerdown")
  vi.advanceTimersByTime(50)
  pointer(button, "pointerup")
  button.click()
  expect(click).not.toHaveBeenCalled()
  expect(confirm).not.toHaveBeenCalled()
  expect(button.querySelector("[data-hold-hint]")?.textContent).toBe(
    "Hold to stop"
  )
  vi.advanceTimersByTime(2000)
  expect(button.textContent).toBe("Stop")

  pointer(button, "pointerdown")
  vi.advanceTimersByTime(HOLD_TO_CONFIRM_MS)
  expect(confirm).toHaveBeenCalledTimes(1)
  expect(click).not.toHaveBeenCalled()
})

it("a Button without it clicks as before, whichever way its handler is given", () => {
  const click = vi.fn()
  const plain = mountButton({ onClick: click })
  expect(plain.querySelector("[data-hold-fill]")).toBeNull()
  pointer(plain, "pointerdown")
  vi.advanceTimersByTime(5000)
  expect(click).not.toHaveBeenCalled()
  plain.click()
  expect(click).toHaveBeenCalledTimes(1)
  expect(click.mock.calls[0][0]).toBeInstanceOf(MouseEvent)
  dispose?.()

  const bound = vi.fn()
  mountButton({ onClick: [bound, "data"] }).click()
  expect(bound).toHaveBeenCalledWith("data", expect.any(MouseEvent))
})

it("a disabled or loading Button cannot be held", () => {
  const confirm = vi.fn()
  const disabled = mountButton({ hold: { onConfirm: confirm }, disabled: true })
  pointer(disabled, "pointerdown")
  vi.advanceTimersByTime(5000)
  dispose?.()
  const loading = mountButton({ hold: { onConfirm: confirm }, loading: true })
  pointer(loading, "pointerdown")
  vi.advanceTimersByTime(5000)
  expect(confirm).not.toHaveBeenCalled()
})
