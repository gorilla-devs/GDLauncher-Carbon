import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { createSignal, For, splitProps } from "solid-js"
import { Dynamic, render } from "solid-js/web"
import type {
  FEMapDimension,
  FEMapPoint,
  FEWorldMap
} from "@gd/core_module/bindings"

vi.mock("@/utils/rspcClient", () => ({
  apiUrl: (path: string) => `http://core${path}&_token=t`
}))
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => <span>{p.key}</span>,
  useTransContext: () => [(key: string) => key]
}))
vi.mock("@gd/ui", async () => {
  const ui = await import("@/test/uiMenus.fixture")
  return {
    ...ui,
    // A native select that labels each option the way the component does.
    Select: (p: any) => (
      <select
        value={p.value}
        onChange={(e) => p.onChange(e.currentTarget.value)}
      >
        <For each={p.options}>
          {(id: string) => (
            <option value={id}>
              {p.itemComponent({ item: { rawValue: id } })}
            </option>
          )}
        </For>
      </select>
    ),
    SelectItem: (p: any) => <>{p.children}</>,
    // Renders what `as` names, with its own attributes.
    TooltipTrigger: (p: any) => {
      const [own, rest] = splitProps(p, ["as", "children"])
      return (
        <Dynamic component={own.as ?? "div"} {...rest}>
          {own.children}
        </Dynamic>
      )
    }
  }
})

import WorldMap from "./WorldMap"

const WIDTH = 1000
const HEIGHT = 600
const OVERWORLD = "minecraft:overworld"
const AETHER = "aether:the_aether"

const region = (x: number, z: number, revision = `r${x}_${z}`) => ({
  x,
  z,
  revision
})
const square = (from: number, to: number) => {
  const out = []
  for (let z = from; z <= to; z++)
    for (let x = from; x <= to; x++) out.push(region(x, z))
  return out
}
const dimension = (
  id: string,
  regions: FEMapDimension["regions"],
  revision = `${id}-rev`
): FEMapDimension => ({ id, ceiling: false, revision, regions })

/** The overworld around the origin plus one region to the east, and the
 *  Aether with one region; the spawn at the origin. */
const sample = (over: Partial<FEWorldMap> = {}): FEWorldMap => ({
  dimensions: [
    dimension(OVERWORLD, [...square(-1, 0), region(2, 0)]),
    dimension(AETHER, [region(0, 0, "ae 0&")])
  ],
  spawn: { dimension: OVERWORLD, x: 0, z: 0 },
  player: null,
  unmappable: null,
  ...over
})

let dispose: (() => void) | undefined

beforeEach(() => {
  vi.stubGlobal(
    "ResizeObserver",
    class {
      constructor(private callback: ResizeObserverCallback) {}
      observe(target: Element) {
        this.callback(
          [{ target, contentRect: { width: WIDTH, height: HEIGHT } } as any],
          this as any
        )
      }
      unobserve() {}
      disconnect() {}
    }
  )
  Element.prototype.setPointerCapture = vi.fn()
  Element.prototype.releasePointerCapture = vi.fn()
  Element.prototype.hasPointerCapture = vi.fn(() => true)
})
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
  document.body.className = ""
  vi.useRealTimers()
  vi.unstubAllGlobals()
})

function mount(
  options: {
    map?: FEWorldMap
    world?: string
    dimension?: string
    filled?: boolean
  } = {}
) {
  const [map, setMap] = createSignal(options.map ?? sample())
  const [world, setWorld] = createSignal(options.world ?? "My World")
  const [dim, setDimension] = createSignal(options.dimension ?? OVERWORLD)
  const [filled, setFilled] = createSignal(options.filled ?? false)
  const onDimension = vi.fn()
  const onFill = vi.fn()
  const host = document.createElement("div")
  document.body.append(host)
  dispose = render(
    () => (
      <WorldMap
        instanceId={5}
        world={world()}
        map={map()}
        dimension={dim()}
        onDimension={onDimension}
        filled={filled()}
        onFill={onFill}
      />
    ),
    host
  )
  return { setMap, setWorld, setDimension, setFilled, onDimension, onFill }
}

const byId = (id: string) =>
  document.querySelector<HTMLElement>(`[data-testid="${id}"]`)
const all = (id: string) => [
  ...document.querySelectorAll<HTMLElement>(`[data-testid="${id}"]`)
]
const panel = () => byId("world-map")!
const pieces = () => all("world-map-piece") as HTMLImageElement[]
const address = (img: HTMLImageElement) =>
  Object.fromEntries(new URL(img.getAttribute("src")!).searchParams)
const at = (img: HTMLImageElement) => {
  const a = address(img)
  return `${a.scale}:${a.x},${a.z}`
}
const asked = () => pieces().map(at).sort()
const layer = () => panel().querySelector<HTMLElement>(".world-map-layer")!
/** The layer's `translate(…px, …px) scale(…)`. */
const transform = () => {
  const m =
    /translate\((-?[\d.e-]+)px, (-?[\d.e-]+)px\) scale\(([\d.e-]+)\)/.exec(
      layer().style.transform
    )!
  return { x: Number(m[1]), y: Number(m[2]), zoom: Number(m[3]) }
}
/** Where a piece is on screen. */
const screen = (img: HTMLElement) => {
  const t = transform()
  return {
    left: t.x + parseFloat(img.style.left) * t.zoom,
    top: t.y + parseFloat(img.style.top) * t.zoom,
    size: parseFloat(img.style.width) * t.zoom
  }
}
const piece = (key: string) => pieces().find((p) => at(p) === key)!

const pointer = (type: string, x: number, y: number, target = panel()) =>
  target.dispatchEvent(
    new PointerEvent(type, {
      clientX: x,
      clientY: y,
      button: 0,
      buttons: type === "pointerup" ? 0 : 1,
      pointerId: 1,
      bubbles: type !== "pointerleave"
    })
  )
const hover = (x: number, y: number) => pointer("pointermove", x, y)
const drag = (from: [number, number], to: [number, number]) => {
  pointer("pointerdown", ...from)
  pointer("pointermove", (from[0] + to[0]) / 2, (from[1] + to[1]) / 2)
  pointer("pointermove", ...to)
  pointer("pointerup", ...to)
}
const wheel = (deltaY: number, x: number, y: number) => {
  const event = new WheelEvent("wheel", {
    deltaY,
    clientX: x,
    clientY: y,
    bubbles: true,
    cancelable: true
  })
  panel().dispatchEvent(event)
  return event
}
const coordinates = () => byId("world-map-coordinates")?.textContent ?? null
/** The block at the middle of the panel. */
const centre = () => {
  hover(WIDTH / 2, HEIGHT / 2)
  return coordinates()
}

it("pieces are asked with the world, dimension and revision in the address, values encoded", () => {
  mount({ world: "a&b c", dimension: AETHER })
  const [img] = pieces()
  const src = img.getAttribute("src")!
  expect(src).toContain("/instance/worldMapPiece?")
  expect(src).toContain("world=a%26b%20c")
  expect(src).toContain("dimension=aether%3Athe_aether")
  expect(src).toContain("rev=ae%200%26")
  expect(address(img)).toEqual({
    instance_id: "5",
    world: "a&b c",
    dimension: AETHER,
    scale: "0",
    x: "0",
    z: "0",
    rev: "ae 0&",
    _token: "t"
  })
  expect(img.getAttribute("alt")).toBe("")
  expect(img.getAttribute("draggable")).toBe("false")
})

it("a piece shows its loading animation until its image loads, and not after", () => {
  mount()
  expect(asked()).toEqual(["0:-1,-1", "0:-1,0", "0:0,-1", "0:0,0"])
  expect(all("world-map-loading")).toHaveLength(4)

  const loaded = piece("0:0,0")
  loaded.dispatchEvent(new Event("load"))
  expect(all("world-map-loading")).toHaveLength(3)
  // The placeholder sits on its piece's own box.
  const boxes = all("world-map-loading").map((l) => screen(l))
  expect(boxes).not.toContainEqual(screen(loaded))
  expect(boxes).toContainEqual(screen(piece("0:-1,-1")))
  expect(pieces()).toContain(loaded)
})

it("a piece that answers nothing is shown as nothing and not asked again", () => {
  mount()
  piece("0:-1,0").dispatchEvent(new Event("error"))
  expect(asked()).toEqual(["0:-1,-1", "0:0,-1", "0:0,0"])
  expect(all("world-map-loading")).toHaveLength(3)
  // Away and back: the same revision is not asked again.
  drag([500, 300], [-700, 300])
  drag([-700, 300], [500, 300])
  expect(asked()).toEqual(["0:-1,-1", "0:0,-1", "0:0,0"])
})

it("an area without a region asks nothing and animates nothing", () => {
  mount({
    map: sample({
      dimensions: [dimension(OVERWORLD, [region(10, 10)])]
    })
  })
  expect(pieces()).toEqual([])
  expect(all("world-map-loading")).toEqual([])
})

it("dragging moves the pieces and asks the newly visible ones", () => {
  mount()
  const before = screen(piece("0:0,0"))
  const element = piece("0:0,0")
  // Region (2, 0) starts at block 1024, beyond the right edge at 500.
  expect(asked()).not.toContain("0:2,0")
  drag([500, 300], [-100, 260])
  expect(asked()).toContain("0:2,0")
  const after = screen(piece("0:0,0"))
  expect(after.left).toBeCloseTo(before.left - 600)
  expect(after.top).toBeCloseTo(before.top - 40)
  // Moved, not made again.
  expect(piece("0:0,0")).toBe(element)
  expect(Element.prototype.setPointerCapture).toHaveBeenCalledWith(1)
})

it("a press that moves less than 3 pixels does not move the map", () => {
  mount()
  const before = transform()
  drag([500, 300], [502, 301])
  expect(transform()).toEqual(before)
})

it("a piece that left the screen is removed", () => {
  mount()
  const west = piece("0:-1,0")
  drag([500, 300], [-100, 300])
  expect(asked()).not.toContain("0:-1,0")
  expect(west.isConnected).toBe(false)
})

it("the wheel zooms around the pointer and changes scale", () => {
  vi.useFakeTimers({ toFake: ["Date"] })
  mount()
  hover(700.5, 150.5)
  const under = coordinates()
  const size = screen(piece("0:0,0")).size
  expect(layer().style.imageRendering).toBe("pixelated")

  const event = wheel(-100, 700.5, 150.5)
  expect(event.defaultPrevented).toBe(true)
  expect(screen(piece("0:0,0")).size).toBeCloseTo(size * 1.25)
  expect(coordinates()).toBe(under)

  // A trackpad's burst inside 40 ms is one step.
  wheel(-3, 700.5, 150.5)
  wheel(-3, 700.5, 150.5)
  expect(transform().zoom).toBeCloseTo(1.25)

  const notch = (deltaY: number) => {
    vi.setSystemTime(Date.now() + 50)
    wheel(deltaY, 700.5, 150.5)
  }
  notch(100)
  notch(100)
  // 0.8 pixels per block: still one pixel per block, smoothed.
  expect(transform().zoom).toBeCloseTo(0.8)
  expect(asked().every((p) => p.startsWith("0:"))).toBe(true)
  expect(layer().style.imageRendering).not.toBe("pixelated")
  notch(100)
  // 0.64: pieces of two blocks a pixel, each pixel above a screen pixel.
  expect(transform().zoom).toBeCloseTo(0.64)
  expect(asked().length).toBeGreaterThan(0)
  expect(asked().every((p) => p.startsWith("1:"))).toBe(true)
  expect(layer().style.imageRendering).toBe("pixelated")
  expect(coordinates()).toBe(under)

  // Out no further than the whole dimension: 4 regions wide in 1000 pixels.
  for (let i = 0; i < 20; i++) notch(100)
  expect(transform().zoom).toBeCloseTo(WIDTH / (4 * 512))
  for (let i = 0; i < 40; i++) notch(-100)
  expect(transform().zoom).toBe(8)
})

it("double click zooms in by two around the pointer", () => {
  mount()
  hover(300.5, 400.5)
  const under = coordinates()
  panel().dispatchEvent(
    new MouseEvent("dblclick", {
      clientX: 300.5,
      clientY: 400.5,
      bubbles: true
    })
  )
  expect(transform().zoom).toBeCloseTo(2)
  expect(coordinates()).toBe(under)
})

it("the map opens on the player, else the spawn, else everything", () => {
  const player: FEMapPoint = { dimension: OVERWORLD, x: -700, z: 300 }
  mount({
    map: sample({ player, spawn: { dimension: OVERWORLD, x: 5, z: 6 } })
  })
  expect(centre()).toBe("-700, 300")
  expect(transform().zoom).toBe(1)
  dispose!()

  mount({
    map: sample({
      player: { ...player, dimension: "minecraft:the_nether" },
      spawn: { dimension: OVERWORLD, x: 5, z: 6 }
    })
  })
  expect(centre()).toBe("5, 6")
  expect(transform().zoom).toBe(1)
  dispose!()

  mount({ map: sample({ spawn: null }) })
  // Regions -1..2 by -1..0: the middle is block (512, 0).
  expect(centre()).toBe("512, 0")
  expect(transform().zoom).toBeCloseTo(WIDTH / (4 * 512))
})

it("markers are shown in their own dimension only", () => {
  const view = mount({
    map: sample({
      spawn: { dimension: OVERWORLD, x: 100, z: -50 },
      player: { dimension: AETHER, x: 3, z: 4 }
    })
  })
  // The map opens on the spawn: its block is at the middle.
  const spawn = byId("world-map-spawn")!
  expect(spawn.className).toContain("i-hugeicons:home-01")
  expect(byId("world-map-player")).toBeNull()
  expect(layer().contains(spawn)).toBe(false)
  const placed = () => ({
    left: parseFloat(byId("world-map-spawn")!.style.left),
    top: parseFloat(byId("world-map-spawn")!.style.top)
  })
  expect(placed()).toEqual({ left: 500, top: 300 })
  expect(spawn.closest("[data-testid='world-map']")!.textContent).toContain(
    "content:_trn_worlds.map.spawn"
  )

  drag([500, 300], [400, 250])
  expect(placed()).toEqual({ left: 400, top: 250 })

  view.setDimension(AETHER)
  expect(byId("world-map-spawn")).toBeNull()
  expect(byId("world-map-player")!.className).toContain(
    "i-hugeicons:location-01"
  )
  expect(panel().textContent).toContain("content:_trn_worlds.map.player")
})

it("coordinates follow the pointer and vanish when it leaves", () => {
  mount()
  expect(coordinates()).toBeNull()
  hover(510, 300)
  expect(coordinates()).toBe("10, 0")
  hover(480, 280)
  expect(coordinates()).toBe("-20, -20")
  pointer("pointerleave", 480, 280)
  expect(coordinates()).toBeNull()
})

it("the dropdown lists the dimensions and is absent with one", () => {
  const view = mount()
  const select = byId("world-map-dimension")!.querySelector("select")!
  expect([...select.options].map((o) => o.textContent)).toEqual([
    "content:_trn_worlds.map.overworld",
    AETHER
  ])
  select.value = AETHER
  select.dispatchEvent(new Event("change", { bubbles: true }))
  expect(view.onDimension).toHaveBeenCalledWith(AETHER)
  dispose!()

  mount({
    map: sample({ dimensions: [dimension(OVERWORLD, square(-1, 0))] })
  })
  expect(byId("world-map-dimension")).toBeNull()
})

it("the fill icon reports the toggle", () => {
  const view = mount()
  byId("world-map-fill")!.click()
  expect(view.onFill).toHaveBeenLastCalledWith(true)
  expect(panel().textContent).toContain("content:_trn_worlds.map.fill")
  view.setFilled(true)
  byId("world-map-fill")!.click()
  expect(view.onFill).toHaveBeenLastCalledWith(false)
  expect(panel().textContent).toContain("content:_trn_worlds.map.shrink")
  // Pressing the icon does not start a drag.
  pointer("pointerdown", 990, 10, byId("world-map-fill")!)
  expect(Element.prototype.setPointerCapture).not.toHaveBeenCalled()
})

it("an equal map after a refetch keeps the view and asks again only what changed", () => {
  const view = mount()
  drag([500, 300], [450, 300])
  const moved = transform()
  const kept = piece("0:0,0")
  kept.dispatchEvent(new Event("load"))
  view.setMap(sample())
  expect(transform()).toEqual(moved)
  expect(piece("0:0,0")).toBe(kept)
  expect(all("world-map-loading")).toHaveLength(3)

  const changed = sample()
  changed.dimensions[0].regions[0] = region(-1, -1, "played")
  view.setMap(changed)
  expect(transform()).toEqual(moved)
  expect(address(piece("0:-1,-1")).rev).toBe("played")
  expect(piece("0:0,0")).toBe(kept)
})

it("another world or dimension starts over", () => {
  const view = mount()
  drag([500, 300], [450, 300])
  piece("0:0,0").dispatchEvent(new Event("error"))
  view.setWorld("Other")
  expect(centre()).toBe("0, 0")
  expect(asked()).toContain("0:0,0")
  expect(address(piece("0:0,0")).world).toBe("Other")
  // Back again, what answered nothing before is asked again.
  view.setWorld("My World")
  expect(address(piece("0:0,0")).world).toBe("My World")

  view.setDimension(AETHER)
  expect(asked()).toEqual(["0:0,0"])
  expect(address(piece("0:0,0")).dimension).toBe(AETHER)
})

it("no animation under potato-pc", () => {
  mount()
  const [loading] = all("world-map-loading")
  expect(getComputedStyle(loading).animationName).toBe("world-map-sheen")
  document.body.classList.add("potato-pc")
  expect(getComputedStyle(loading).animationName).toBe("none")
})
