import { afterEach, beforeEach, expect, test, vi } from "vitest"
import { render } from "solid-js/web"

const control = vi.hoisted(() => ({
  made: [] as any[],
  wasmUrl: undefined as string | undefined
}))
vi.mock("@rive-app/canvas", () => ({
  RuntimeLoader: {
    setWasmUrl: (url: string) => {
      control.wasmUrl = url
    }
  },
  Rive: class {
    resizeDrawingSurfaceToCanvas = vi.fn()
    stopRendering = vi.fn()
    cleanup = vi.fn()
    constructor(public options: any) {
      control.made.push(this)
    }
  }
}))
import RiveAppWrapper from "./RiveAppWrapper"

let dispose: (() => void) | undefined
const stopped = vi.fn()
const animation = () => control.made[0].options
const ended = () => animation().onStateChange({ data: [""] })

beforeEach(() => {
  vi.useFakeTimers()
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => <RiveAppWrapper src="intro.riv" onStop={stopped} />,
    root
  )
})
afterEach(() => {
  dispose?.()
  document.body.textContent = ""
  control.made = []
  stopped.mockClear()
  vi.useRealTimers()
})

test("the animation engine is the one shipped with the app, not one from a CDN", () => {
  expect(control.wasmUrl).toBeTruthy()
  expect(control.wasmUrl).not.toMatch(/^https?:\/\/(?!localhost)/)
  expect(control.wasmUrl).toMatch(/rive.*\.wasm/)
})

test("an animation that plays to its end stops once", () => {
  animation().onLoad()
  vi.advanceTimersByTime(4000)
  expect(stopped).not.toHaveBeenCalled()

  ended()
  expect(stopped).toHaveBeenCalledOnce()

  vi.advanceTimersByTime(60_000)
  expect(stopped).toHaveBeenCalledOnce()
})

test("an animation that never loads stops after four seconds", () => {
  vi.advanceTimersByTime(3999)
  expect(stopped).not.toHaveBeenCalled()

  vi.advanceTimersByTime(1)
  expect(stopped).toHaveBeenCalledOnce()
})

test("an animation whose file cannot be read stops at once", () => {
  animation().onLoadError()
  expect(stopped).toHaveBeenCalledOnce()

  vi.advanceTimersByTime(60_000)
  expect(stopped).toHaveBeenCalledOnce()
})

test("an animation that loaded but never ends stops after fifteen seconds", () => {
  vi.advanceTimersByTime(3000)
  animation().onLoad()
  vi.advanceTimersByTime(14_999)
  expect(stopped).not.toHaveBeenCalled()

  vi.advanceTimersByTime(1)
  expect(stopped).toHaveBeenCalledOnce()
})

test("an animation taken off the screen stops nothing later", () => {
  dispose?.()
  dispose = undefined
  vi.advanceTimersByTime(60_000)
  expect(stopped).not.toHaveBeenCalled()
})
