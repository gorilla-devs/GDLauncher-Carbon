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
/** The panel's size the stubbed observer reports. */
let panelSize = { width: WIDTH, height: HEIGHT }
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
let frames = new Map<number, FrameRequestCallback>()
let nextFrameId = 1
/** Runs the animation frames asked so far. */
const frame = () => {
  const due = frames
  frames = new Map()
  for (const callback of due.values()) callback(performance.now())
}

beforeEach(() => {
  panelSize = { width: WIDTH, height: HEIGHT }
  vi.stubGlobal(
    "ResizeObserver",
    class {
      constructor(private callback: ResizeObserverCallback) {}
      observe(target: Element) {
        this.callback(
          [{ target, contentRect: { ...panelSize } } as any],
          this as any
        )
      }
      unobserve() {}
      disconnect() {}
    }
  )
  vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) => {
    const id = nextFrameId++
    frames.set(id, callback)
    return id
  })
  vi.stubGlobal("cancelAnimationFrame", (id: number) => frames.delete(id))
  Element.prototype.setPointerCapture = vi.fn()
  Element.prototype.releasePointerCapture = vi.fn()
  Element.prototype.hasPointerCapture = vi.fn(() => true)
})
afterEach(() => {
  dispose?.()
  dispose = undefined
  frames = new Map()
  document.body.textContent = ""
  document.body.className = ""
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
const addressOf = (src: string) => Object.fromEntries(new URL(src).searchParams)
const address = (img: HTMLImageElement) => addressOf(img.getAttribute("src")!)
const keyOf = (a: Record<string, string>) => `${a.scale}:${a.x},${a.z}`
const at = (img: HTMLImageElement) => keyOf(address(img))
const asked = () => pieces().map(at).sort()
const layer = () => panel().querySelector<HTMLElement>(".world-map-layer")!
/** Where an element is on screen, in pixels. */
const screen = (element: HTMLElement) => ({
  left: parseFloat(element.style.left),
  top: parseFloat(element.style.top),
  width: parseFloat(element.style.width),
  height: parseFloat(element.style.height)
})
const piece = (key: string) => pieces().find((p) => at(p) === key)!
/** Screen pixels per block, read from the last piece drawn (it is of the
 *  scale shown). Exact to a pixel in a piece's width. */
const zoom = () => {
  const img = pieces().at(-1)!
  return screen(img).width / (512 * 2 ** Number(address(img).scale))
}
const load = (img: HTMLImageElement) => img.dispatchEvent(new Event("load"))
/** Every address an image of the map carried since this was called. */
const watchAddresses = () => {
  const seen = new Set<string>()
  const observer = new MutationObserver(() => {})
  observer.observe(panel(), {
    subtree: true,
    childList: true,
    attributes: true,
    attributeFilter: ["src"],
    attributeOldValue: true
  })
  return () => {
    for (const record of observer.takeRecords()) {
      if (record.oldValue) seen.add(record.oldValue)
      const nodes =
        record.type === "attributes" ? [record.target] : [...record.addedNodes]
      for (const node of nodes) {
        if (!(node instanceof Element)) continue
        for (const img of [node, ...node.querySelectorAll("img")]) {
          const src = img.getAttribute("src")
          if (img.tagName === "IMG" && src) seen.add(src)
        }
      }
    }
    return [...seen].map(addressOf)
  }
}

const pointer = (
  type: string,
  x: number,
  y: number,
  options: { target?: HTMLElement; buttons?: number } = {}
) =>
  (options.target ?? panel()).dispatchEvent(
    new PointerEvent(type, {
      clientX: x,
      clientY: y,
      button: 0,
      buttons:
        options.buttons ??
        (["pointerup", "pointercancel", "lostpointercapture"].includes(type)
          ? 0
          : 1),
      pointerId: 1,
      bubbles: type !== "pointerleave"
    })
  )
const hover = (x: number, y: number) => pointer("pointermove", x, y)
const drag = (from: [number, number], to: [number, number]) => {
  pointer("pointerdown", ...from)
  pointer("pointermove", (from[0] + to[0]) / 2, (from[1] + to[1]) / 2)
  frame()
  pointer("pointermove", ...to)
  frame()
  pointer("pointerup", ...to)
}
const wheel = (
  deltaY: number,
  x: number,
  y: number,
  deltaMode: number = WheelEvent.DOM_DELTA_PIXEL
) => {
  const event = new WheelEvent("wheel", {
    deltaY,
    deltaMode,
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
const grid = (xs: [number, number], zs: [number, number]) => {
  const out = []
  for (let z = zs[0]; z <= zs[1]; z++)
    for (let x = xs[0]; x <= xs[1]; x++) out.push(region(x, z))
  return out
}

it("pieces are asked with the world, dimension and revision in the address, values encoded", () => {
  mount({ world: "a&b c#?", dimension: AETHER })
  const [img] = pieces()
  const src = img.getAttribute("src")!
  expect(src).toContain("/instance/worldMapPiece?")
  expect(src).toContain("world=a%26b%20c%23%3F&")
  expect(src).toContain("dimension=aether%3Athe_aether")
  expect(src).toContain("rev=ae%200%26")
  expect(address(img)).toEqual({
    instance_id: "5",
    world: "a&b c#?",
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
  load(loaded)
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

it("pieces sit in whole screen pixels, not in a scaled layer", () => {
  mount()
  expect(layer().style.transform).toBe("")
  for (const img of pieces()) {
    const box = screen(img)
    for (const value of Object.values(box))
      expect(Number.isInteger(value)).toBe(true)
  }
  // Neighbours share their edge.
  const west = screen(piece("0:-1,0"))
  expect(west.left + west.width).toBe(screen(piece("0:0,0")).left)
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
  expect(after.left).toBe(before.left - 600)
  expect(after.top).toBe(before.top - 40)
  // Moved, not made again.
  expect(piece("0:0,0")).toBe(element)
  expect(Element.prototype.setPointerCapture).toHaveBeenCalledWith(1)
})

it("a drag moves the pieces once per animation frame", () => {
  mount()
  const before = screen(piece("0:0,0"))
  pointer("pointerdown", 500, 300)
  pointer("pointermove", 450, 300)
  pointer("pointermove", 400, 300)
  expect(screen(piece("0:0,0"))).toEqual(before)
  frame()
  expect(screen(piece("0:0,0")).left).toBe(before.left - 100)
})

it("a press that moves less than 3 pixels does not move the map", () => {
  mount()
  const before = screen(piece("0:0,0"))
  drag([500, 300], [502, 301])
  expect(screen(piece("0:0,0"))).toEqual(before)
})

it.each(["pointerup", "pointercancel", "lostpointercapture"])(
  "a drag ends on %s",
  (type) => {
    mount()
    pointer("pointerdown", 500, 300)
    pointer("pointermove", 450, 300)
    frame()
    const before = screen(piece("0:0,0"))
    pointer(type, 450, 300)
    pointer("pointermove", 300, 300)
    frame()
    expect(screen(piece("0:0,0"))).toEqual(before)
  }
)

it("a drag ends on a move with no button pressed", () => {
  mount()
  pointer("pointerdown", 500, 300)
  pointer("pointermove", 450, 300)
  frame()
  const before = screen(piece("0:0,0"))
  pointer("pointermove", 400, 300, { buttons: 0 })
  frame()
  pointer("pointermove", 300, 300)
  frame()
  expect(screen(piece("0:0,0"))).toEqual(before)
})

it("a wheel turn during a drag keeps its zoom and the drag goes on from there", () => {
  mount()
  hover(500, 300)
  const grabbed = coordinates()
  pointer("pointerdown", 500, 300)
  pointer("pointermove", 400, 300)
  frame()
  wheel(-100, 400, 300)
  frame()
  expect(zoom()).toBeCloseTo(1.25)
  pointer("pointermove", 300, 300)
  frame()
  expect(zoom()).toBeCloseTo(1.25)
  pointer("pointerup", 300, 300)
  // The block grabbed is still under the pointer.
  hover(300, 300)
  expect(coordinates()).toBe(grabbed)
})

it("a piece that left the screen is removed and its request dropped", () => {
  mount()
  const west = piece("0:-1,0")
  drag([500, 300], [-100, 300])
  expect(asked()).not.toContain("0:-1,0")
  expect(west.isConnected).toBe(false)
  expect(west.hasAttribute("src")).toBe(false)
  // What a browser may report for the dropped request is not a failure.
  west.dispatchEvent(new Event("error"))
  drag([-100, 300], [500, 300])
  expect(asked()).toContain("0:-1,0")
})

it("closing the map drops the requests of its pieces", () => {
  mount()
  const shown = pieces()
  expect(shown).toHaveLength(4)
  dispose!()
  dispose = undefined
  for (const img of shown) expect(img.hasAttribute("src")).toBe(false)
})

it("at most four pieces load at a time, nearest the centre first", () => {
  panelSize = { width: 2000, height: 1200 }
  mount({
    map: sample({ dimensions: [dimension(OVERWORLD, grid([-2, 1], [-1, 1]))] })
  })
  // Twelve pieces on screen, each waiting or loading under its placeholder.
  expect(all("world-map-loading")).toHaveLength(12)
  expect(pieces().filter((p) => p.hasAttribute("src"))).toHaveLength(4)
  expect(asked()).toEqual(["0:-1,-1", "0:-1,0", "0:0,-1", "0:0,0"])

  load(piece("0:0,0"))
  expect(asked()).toEqual(["0:-1,-1", "0:-1,0", "0:0,-1", "0:0,0", "0:1,0"])
  expect(all("world-map-loading")).toHaveLength(11)
})

it("a waiting piece that leaves the view never loads", () => {
  panelSize = { width: 2000, height: 1200 }
  mount({
    map: sample({ dimensions: [dimension(OVERWORLD, grid([-2, 1], [-1, 1]))] })
  })
  const seen = watchAddresses()
  // The westmost column, farthest from the centre, leaves the screen.
  drag([1000, 600], [400, 600])
  const done = new Set<HTMLImageElement>()
  for (let round = 0; round < 20; round++)
    for (const img of pieces())
      if (!done.has(img)) {
        done.add(img)
        load(img)
      }
  expect(all("world-map-loading")).toEqual([])
  expect(seen().some((a) => a.x === "-2")).toBe(false)
  expect(asked()).toHaveLength(9)
})

it("switching the world never asks the new world's pieces for the old view", () => {
  const view = mount()
  drag([500, 300], [-700, 300])
  expect(asked()).toEqual(["0:2,0"])
  const seen = watchAddresses()
  view.setWorld("Other")
  expect(asked()).toEqual(["0:-1,-1", "0:-1,0", "0:0,-1", "0:0,0"])
  const asOther = seen()
    .filter((a) => a.world === "Other")
    .map(keyOf)
    .sort()
  expect(asOther).toEqual(asked())
})

it("the wheel zooms around the pointer and changes scale", () => {
  mount()
  hover(700.5, 150.5)
  const under = coordinates()
  const size = screen(piece("0:0,0")).width
  expect(layer().style.imageRendering).toBe("pixelated")

  const event = wheel(-100, 700.5, 150.5)
  expect(event.defaultPrevented).toBe(true)
  frame()
  expect(screen(piece("0:0,0")).width).toBeCloseTo(size * 1.25, 0)
  expect(coordinates()).toBe(under)

  const notch = (deltaY: number) => {
    wheel(deltaY, 700.5, 150.5)
    frame()
  }
  notch(100)
  notch(100)
  // 0.8 pixels per block: still one pixel per block, smoothed.
  expect(zoom()).toBeCloseTo(0.8)
  expect(asked().every((p) => p.startsWith("0:"))).toBe(true)
  expect(layer().style.imageRendering).not.toBe("pixelated")
  notch(100)
  // 0.64: pieces of two blocks a pixel, each pixel above a screen pixel.
  expect(zoom()).toBeCloseTo(0.64)
  expect(asked().length).toBeGreaterThan(0)
  expect(asked().every((p) => p.startsWith("1:"))).toBe(true)
  expect(layer().style.imageRendering).toBe("pixelated")
  expect(coordinates()).toBe(under)

  // Out no further than the whole dimension: 4 regions wide in 1000 pixels.
  for (let i = 0; i < 20; i++) notch(100)
  expect(zoom()).toBeCloseTo(WIDTH / (4 * 512))
  for (let i = 0; i < 40; i++) notch(-100)
  expect(zoom()).toBe(8)
})

it("the wheel zooms by its distance, a notch of 100 being 1.25", () => {
  mount()
  hover(700.5, 150.5)
  const under = coordinates()
  wheel(-100, 700.5, 150.5)
  frame()
  expect(zoom()).toBeCloseTo(1.25, 2)

  // A trackpad's stream inside one frame adds up to the same.
  for (let i = 0; i < 10; i++) wheel(-10, 700.5, 150.5)
  frame()
  expect(zoom()).toBeCloseTo(1.25 ** 2, 2)

  // Lines are 40 pixels each.
  wheel(-3, 700.5, 150.5, WheelEvent.DOM_DELTA_LINE)
  frame()
  expect(zoom()).toBeCloseTo(1.25 ** 3.2, 2)

  // One small turn is a small step.
  wheel(10, 700.5, 150.5)
  frame()
  expect(zoom()).toBeCloseTo(1.25 ** 3.1, 2)
  expect(coordinates()).toBe(under)
})

it("crossing into another scale keeps the loaded pieces until the new ones load", () => {
  mount()
  const old = pieces()
  for (const img of old.slice(0, 3)) load(img)
  const unloaded = old[3]
  for (let i = 0; i < 3; i++) {
    wheel(100, 500, 300)
    frame()
  }
  // 0.512 pixels per block: scale 1.
  expect(asked().filter((p) => p.startsWith("1:"))).toHaveLength(4)
  const kept = old.slice(0, 3)
  for (const img of kept) expect(img.isConnected).toBe(true)
  expect(unloaded.isConnected).toBe(false)
  // Drawn above the new placeholders and beneath the new pieces, so they
  // show until the new pieces cover them.
  const placeholders = all("world-map-loading")
  expect(placeholders).toHaveLength(4)
  const fresh = pieces().filter((p) => address(p).scale === "1")
  for (const img of kept) {
    for (const l of placeholders)
      expect(
        l.compareDocumentPosition(img) & Node.DOCUMENT_POSITION_FOLLOWING
      ).toBeTruthy()
    for (const f of fresh)
      expect(
        img.compareDocumentPosition(f) & Node.DOCUMENT_POSITION_FOLLOWING
      ).toBeTruthy()
  }

  // The new piece over an old one loads: that old one goes.
  const covering = piece("1:0,-1")
  const under = kept.find((img) => at(img) === "0:0,-1")!
  load(covering)
  expect(under.isConnected).toBe(false)
  expect(kept.filter((img) => img.isConnected)).toHaveLength(2)

  for (const img of fresh) load(img)
  for (const img of kept) expect(img.isConnected).toBe(false)
  expect(asked().every((p) => p.startsWith("1:"))).toBe(true)
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
  frame()
  expect(zoom()).toBeCloseTo(2)
  expect(coordinates()).toBe(under)
})

it("the map opens on the player, else the spawn, else everything", () => {
  const player: FEMapPoint = { dimension: OVERWORLD, x: -700, z: 300 }
  mount({
    map: sample({ player, spawn: { dimension: OVERWORLD, x: 5, z: 6 } })
  })
  expect(centre()).toBe("-700, 300")
  expect(zoom()).toBe(1)
  dispose!()

  mount({
    map: sample({
      player: { ...player, dimension: "minecraft:the_nether" },
      spawn: { dimension: OVERWORLD, x: 5, z: 6 }
    })
  })
  expect(centre()).toBe("5, 6")
  expect(zoom()).toBe(1)
  dispose!()

  mount({ map: sample({ spawn: null }) })
  // Regions -1..2 by -1..0: the middle is block (512, 0).
  expect(centre()).toBe("512, 0")
  expect(zoom()).toBeCloseTo(WIDTH / (4 * 512))
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
  pointer("pointerdown", 990, 10, { target: byId("world-map-fill")! })
  expect(Element.prototype.setPointerCapture).not.toHaveBeenCalled()
})

it("an equal map after a refetch keeps the view and asks again only what changed", () => {
  const view = mount()
  drag([500, 300], [450, 300])
  const moved = screen(piece("0:0,0"))
  const kept = piece("0:0,0")
  load(kept)
  view.setMap(sample())
  expect(screen(piece("0:0,0"))).toEqual(moved)
  expect(piece("0:0,0")).toBe(kept)
  expect(all("world-map-loading")).toHaveLength(3)

  const changed = sample()
  changed.dimensions[0].regions[0] = region(-1, -1, "played")
  view.setMap(changed)
  expect(screen(piece("0:0,0"))).toEqual(moved)
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
  // Back again, the view starts over and what answered nothing before is
  // asked again.
  view.setWorld("My World")
  expect(centre()).toBe("0, 0")
  expect(address(piece("0:0,0")).world).toBe("My World")

  view.setDimension(AETHER)
  expect(asked()).toEqual(["0:0,0"])
  expect(address(piece("0:0,0")).dimension).toBe(AETHER)
})

it("the sheen moves by transform, and not under potato-pc", () => {
  mount()
  const [loading] = all("world-map-loading")
  const sheen = loading.firstElementChild!
  expect(getComputedStyle(sheen).animationName).toBe("world-map-sheen")
  const keyframes = [...document.styleSheets]
    .flatMap((sheet) => [...sheet.cssRules])
    // jsdom has no CSSKeyframesRule class; 7 is its rule type.
    .find(
      (rule) =>
        rule.type === 7 && (rule as CSSKeyframesRule).name === "world-map-sheen"
    )!
  expect(keyframes.cssText).toContain("transform")
  expect(keyframes.cssText).not.toContain("background")
  document.body.classList.add("potato-pc")
  expect(getComputedStyle(sheen).animationName).toBe("none")
})
