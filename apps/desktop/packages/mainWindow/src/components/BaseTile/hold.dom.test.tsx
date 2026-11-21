import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { createSignal } from "solid-js"
import { render } from "solid-js/web"
import { BaseTile } from "./index"

vi.mock("@gd/ui", async () => {
  const fixture = await import("@/test/uiFormsRealButton.fixture")
  return { ...fixture, PRESS_CLASSES_LIGHT: "" }
})
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => <span>{p.key}</span>,
  useTransContext: () => [(k: string) => k]
}))
vi.mock("@gd/i18n/helpers", () => ({ getTaskTranslationKey: (k: string) => k }))

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

const mount = (props: {
  isRunning: boolean | (() => boolean)
  hint?: string
  onPlay?: () => void
}) => {
  const play = vi.fn(props.onPlay)
  const open = vi.fn()
  const drag = vi.fn()
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => (
      <BaseTile
        name="Vanilla 1.21"
        size={2}
        isLoading={false}
        isWaiting={false}
        isRunning={
          typeof props.isRunning === "function"
            ? props.isRunning()
            : props.isRunning
        }
        isBusy={false}
        isDeleting={false}
        isMultiSelected={false}
        showCheckbox={false}
        isDragging={false}
        isDragActive={false}
        canDrag
        onClick={open}
        onPlay={play}
        onDragStart={drag}
        shouldSetViewTransition={false}
        viewTransitionPrefix="synthetic"
        playButtonContent={
          (
            typeof props.isRunning === "function"
              ? props.isRunning()
              : props.isRunning
          )
            ? "STOP"
            : "PLAY"
        }
        holdToStopHint={props.hint}
        infoContent="Info"
      />
    ),
    root
  )
  const control = root.querySelector<HTMLElement>(
    '[data-testid="tile-play-control"]'
  )!
  const pointer = (type: string) =>
    control.dispatchEvent(
      new PointerEvent(type, { button: 0, bubbles: true, cancelable: true })
    )
  return { control, pointer, play, open, drag }
}

it("the Stop of a running tile acts only when held, and a click on it shows the hint", () => {
  const tile = mount({ isRunning: true, hint: "Hold to stop" })
  expect(tile.control.textContent).toBe("STOP")
  tile.pointer("pointerdown")
  vi.advanceTimersByTime(60)
  tile.pointer("pointerup")
  tile.control.click()
  expect(tile.play).not.toHaveBeenCalled()
  expect(tile.control.textContent).toBe("Hold to stop")
  vi.advanceTimersByTime(2000)
  expect(tile.control.textContent).toBe("STOP")

  tile.pointer("pointerdown")
  vi.advanceTimersByTime(1199)
  expect(tile.play).not.toHaveBeenCalled()
  vi.advanceTimersByTime(1)
  expect(tile.play).toHaveBeenCalledTimes(1)
  // Neither opens the instance nor starts dragging the tile.
  expect(tile.open).not.toHaveBeenCalled()
  expect(tile.drag).not.toHaveBeenCalled()
})

it("letting go after holding Stop does not press the Play it has become", () => {
  const [running, setRunning] = createSignal(true)
  const tile = mount({
    isRunning: running,
    hint: "Hold to stop",
    // The game stops: the control is Play from here on.
    onPlay: () => setRunning(false)
  })
  tile.pointer("pointerdown")
  vi.advanceTimersByTime(1200)
  expect(tile.play).toHaveBeenCalledTimes(1)
  expect(tile.control.textContent).toBe("PLAY")
  // It stops being a focusable Stop while the mouse is still down.
  tile.control.dispatchEvent(new FocusEvent("blur"))
  vi.advanceTimersByTime(700)
  tile.pointer("pointerup")
  tile.control.click()
  expect(tile.play).toHaveBeenCalledTimes(1)
  // A click of its own, later, plays.
  vi.advanceTimersByTime(1)
  tile.control.click()
  expect(tile.play).toHaveBeenCalledTimes(2)
})

it("Play is still one click", () => {
  const tile = mount({ isRunning: false, hint: "Hold to stop" })
  tile.pointer("pointerdown")
  vi.advanceTimersByTime(5000)
  expect(tile.play).not.toHaveBeenCalled()
  tile.control.click()
  expect(tile.play).toHaveBeenCalledTimes(1)
  expect(tile.open).not.toHaveBeenCalled()
})

it("a tile that does not ask for it stops on one click, as a server's does", () => {
  const tile = mount({ isRunning: true })
  tile.control.click()
  expect(tile.play).toHaveBeenCalledTimes(1)
})
