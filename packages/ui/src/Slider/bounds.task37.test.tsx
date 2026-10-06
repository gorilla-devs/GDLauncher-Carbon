import { afterEach, expect, it } from "vitest"
import { render } from "solid-js/web"
import { Slider } from "./index"

let dispose: (() => void) | undefined
afterEach(() => {
  dispose?.()
  document.body.textContent = ""
})
function fixture(props: any) {
  const changes: number[] = [],
    releases: number[] = []
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => (
      <Slider
        {...props}
        onChange={(v) => changes.push(v)}
        OnRelease={(v) => releases.push(v)}
      />
    ),
    root
  )
  const track = root.querySelector<HTMLDivElement>(".cursor-pointer")!
  const handle = root.querySelector<HTMLDivElement>(".cursor-grab")!
  Object.defineProperties(track, {
    clientWidth: { value: 100 },
    clientHeight: { value: 100 }
  })
  track.getBoundingClientRect = () =>
    ({ left: 10, top: 20, width: 100, height: 100 }) as DOMRect
  function drag(start: number, end = start, target = track) {
    const point = (offset: number) =>
      props.vertical
        ? { clientX: 10, clientY: 20 + offset }
        : { clientX: 10 + offset, clientY: 20 }
    target.dispatchEvent(
      new MouseEvent("mousedown", { bubbles: true, ...point(start) })
    )
    document.dispatchEvent(
      new MouseEvent("mousemove", { bubbles: true, ...point(end) })
    )
    document.dispatchEvent(new MouseEvent("mouseup", { bubbles: true }))
  }
  return { changes, releases, drag, handle }
}
it.each([
  {
    min: 1024,
    max: 8180,
    steps: 1024,
    marks: { 1024: "1", 2048: "2", 4096: "4" }
  },
  { min: 512, max: 8180, steps: 256, marks: { 2048: "2", 4096: "4" } }
])("GPT0263 real RAM endpoint releases stay bounded with min=$min", (props) => {
  const f = fixture(props)
  f.drag(0, 100)
  expect(f.changes).toEqual([8180])
  expect(f.releases).toEqual([8180])
})
it.each([
  [{ min: 1024, max: 8192, steps: 1024 }, 100, 8192],
  [{ min: 1024, max: 8192, steps: 1024 }, 43, 4096],
  [{ min: 512, max: 8180, steps: 256 }, 0, 512],
  [{ min: 0, max: 1, steps: 0.1 }, 26, 0.3],
  [{ min: -1, max: 1, steps: null, marks: { 0: "zero", 1: "one" } }, 45, 0],
  [{ min: 0, max: 10, steps: null, marks: { 2: "two", 8: "eight" } }, 70, 8],
  [{ min: 0, max: 10, steps: null }, 35, 3.5],
  [{ min: 0, max: 9.95, steps: 0.1 }, 100, 9.9],
  [{ min: 0, max: 0.96, steps: 0.1 }, 100, 0.96],
  [{ min: 0, max: 10, steps: null, marks: { 12: "outside" } }, 100, 10],
  [{ min: 1, max: 10, steps: null, marks: { 0: "outside" } }, 0, 1]
] as const)(
  "alignment control %j at %s emits %s to both callbacks",
  (props, position, expected) => {
    const f = fixture(props)
    f.drag(position)
    expect(f.changes).toEqual([expected])
    expect(f.releases).toEqual([expected])
  }
)
it("vertical handle drag uses the same bounded alignment and release path", () => {
  const f = fixture({ min: 512, max: 8180, steps: 256, vertical: true })
  f.drag(0, 100, f.handle)
  expect(f.changes).toEqual([8180])
  expect(f.releases).toEqual([8180])
})
