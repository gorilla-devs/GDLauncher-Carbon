import type {
  FEMapDimension,
  FEMapPoint,
  FEWorldMap
} from "@gd/core_module/bindings"
import { useTransContext, type NamespacedTranslationKey } from "@gd/i18n"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  Tooltip,
  TooltipContent,
  TooltipTrigger
} from "@gd/ui"
import {
  createMemo,
  createSignal,
  For,
  type JSX,
  onCleanup,
  onMount,
  Show,
  untrack
} from "solid-js"
import { createStore } from "solid-js/store"
import { worldMapPieceUrl } from "@/utils/worldMapUrl"
import IconAction from "./IconAction"
import {
  blockAt,
  dragged,
  fitAll,
  MAX_ZOOM,
  pieceBox,
  PIECE_PIXELS,
  scaleFor,
  screenOf,
  visiblePieces,
  zoomAround,
  type View
} from "./mapView"

export interface WorldMapProps {
  instanceId: number
  world: string
  /** What the view belongs to; the world's folder when not given. A renamed
   *  world passes the same value, so its view stays. */
  viewOf?: string
  map: FEWorldMap
  dimension: string
  onDimension: (_id: string) => void
  filled: boolean
  onFill: (_filled: boolean) => void
}

/** A wheel turn of this many pixels zooms by `WHEEL_FACTOR`; smaller and
 *  larger turns zoom in proportion, so a trackpad and a mouse feel alike. */
const WHEEL_FACTOR = 1.25
const WHEEL_NOTCH = 100
const WHEEL_LINE_PIXELS = 40
const WHEEL_PAGE_PIXELS = 800
/** A press that moves less than this is a click, not a drag. */
const DRAG_THRESHOLD = 3
/** Pieces asked of the core at once; the desktop's other calls share the
 *  same few connections. */
const MAX_LOADING = 4

const DIMENSION_NAMES: Record<string, NamespacedTranslationKey> = {
  "minecraft:overworld": "content:_trn_worlds.map.overworld",
  "minecraft:the_nether": "content:_trn_worlds.map.nether",
  "minecraft:the_end": "content:_trn_worlds.map.end"
}

/** One piece asked of the core. The same object stays while its address
 *  does, so the list can change without making its images again. */
interface Asked {
  url: string
  scale: number
  x: number
  z: number
}

/** A dimension refetched with the same revision and regions is the same:
 *  nothing is asked again and the view stays. */
const sameDimension = (
  a: FEMapDimension | undefined,
  b: FEMapDimension | undefined
) =>
  a === b ||
  (!!a &&
    !!b &&
    a.id === b.id &&
    a.revision === b.revision &&
    a.regions.length === b.regions.length &&
    a.regions.every(
      (r, i) =>
        r.x === b.regions[i].x &&
        r.z === b.regions[i].z &&
        r.revision === b.regions[i].revision
    ))

// The sheen moves by transform, which the compositor animates without
// repainting the placeholder.
const SHEEN_STYLE = `
.world-map-loading {
  overflow: hidden;
  background-color: rgb(var(--darkSlate-900));
}
.world-map-sheen {
  position: absolute;
  inset: 0;
  background-image: linear-gradient(100deg, transparent 20%, rgb(var(--darkSlate-700) / 0.6) 50%, transparent 80%);
  transform: translateX(-100%);
  will-change: transform;
  animation-name: world-map-sheen;
  animation-duration: 1.8s;
  animation-timing-function: ease-in-out;
  animation-iteration-count: infinite;
}
@keyframes world-map-sheen {
  from { transform: translateX(-100%); }
  to { transform: translateX(100%); }
}
.potato-pc .world-map-loading {
  background-color: rgb(var(--darkSlate-800));
}
.potato-pc .world-map-sheen {
  animation-name: none;
  display: none;
}
@media (prefers-reduced-motion: reduce) {
  .world-map-loading {
    background-color: rgb(var(--darkSlate-800));
  }
  .world-map-sheen {
    animation-name: none;
    display: none;
  }
}
`

type Settled = "loaded" | "failed"

/** What is drawn: the pieces of the shown scale, and the loaded pieces of
 *  the scale before that stay beneath them until they are covered. */
interface Layout {
  key: string
  scale: number | null
  current: Asked[]
  under: Asked[]
}

const blocksOf = (scale: number) => PIECE_PIXELS * 2 ** scale
const overlaps = (a: Asked, b: Asked) => {
  const sa = blocksOf(a.scale)
  const sb = blocksOf(b.scale)
  return (
    a.x * sa < (b.x + 1) * sb &&
    b.x * sb < (a.x + 1) * sa &&
    a.z * sa < (b.z + 1) * sb &&
    b.z * sb < (a.z + 1) * sa
  )
}
const sameSet = (a: Set<string>, b: Set<string>) =>
  a.size === b.size && [...a].every((v) => b.has(v))

/** One piece's image. A detached image still loads: on removal its address
 *  is taken away, so the browser drops the request and the core stops
 *  painting it, and what the browser reports for that is ignored. */
const PieceImage = (props: {
  url: string
  style: JSX.CSSProperties
  onSettled: (_answer: Settled) => void
}) => {
  let img!: HTMLImageElement
  let dropped = false
  onCleanup(() => {
    dropped = true
    img.removeAttribute("src")
  })
  return (
    <img
      ref={img}
      data-testid="world-map-piece"
      src={props.url}
      alt=""
      draggable={false}
      class="absolute block"
      style={{ ...props.style, "max-width": "none" }}
      onLoad={() => {
        if (!dropped) props.onSettled("loaded")
      }}
      onError={() => {
        if (!dropped) props.onSettled("failed")
      }}
    />
  )
}

const WorldMap = (props: WorldMapProps) => {
  const [t] = useTransContext()
  let root!: HTMLDivElement

  const [size, setSize] = createSignal(
    { width: 0, height: 0 },
    { equals: (a, b) => a.width === b.width && a.height === b.height }
  )
  const [pointerAt, setPointerAt] = createSignal<{
    x: number
    y: number
  } | null>(null)

  const shown = createMemo(
    () => props.map.dimensions.find((d) => d.id === props.dimension),
    undefined,
    { equals: sameDimension }
  )
  const regionRevisions = createMemo(
    () => new Map(shown()?.regions.map((r) => [`${r.x},${r.z}`, r.revision]))
  )
  const limits = createMemo(() => ({
    min: Math.min(
      1,
      fitAll(shown()?.regions ?? [], size().width, size().height).zoom
    ),
    max: MAX_ZOOM
  }))

  // Another world or dimension starts over. Everything below derives from
  // this key in the same update, so no piece is ever asked for the new world
  // at the old view.
  const viewKey = createMemo(() =>
    JSON.stringify([props.viewOf ?? props.world, props.dimension])
  )
  const hasSize = createMemo(() => size().width > 0 && size().height > 0)
  const firstView = (): View => {
    const inShown = (p: FEMapPoint | null) =>
      p?.dimension === props.dimension ? p : null
    const start = inShown(props.map.player) ?? inShown(props.map.spawn)
    if (start)
      return { centerX: start.x + 0.5, centerZ: start.z + 0.5, zoom: 1 }
    return fitAll(shown()?.regions ?? [], size().width, size().height)
  }
  /** The view the map opens on; a new object on every start over. A panel
   *  hidden for a while keeps its view. */
  const opening = createMemo<{ key: string; view: View } | null>((prev) => {
    const key = viewKey()
    if (prev?.key === key) return prev
    return hasSize() ? { key, view: untrack(firstView) } : null
  }, null)
  const [moved, setMoved] = createSignal<{
    from: { key: string; view: View }
    view: View
  } | null>(null)
  const view = createMemo(() => {
    const start = opening()
    const m = moved()
    if (!start) return null
    return m?.from === start ? m.view : start.view
  })
  const setView = (v: View) => {
    const from = opening()
    if (from) setMoved({ from, view: v })
  }

  // What answered, per start: back on a world, what failed is asked again.
  const settled = createMemo(() => {
    viewKey()
    return createStore<Record<string, Settled>>({})
  })

  const layout = createMemo<Layout>(
    (prev) => {
      const key = viewKey()
      const v = view()
      const dimension = shown()
      const [state] = settled()
      if (!v || !dimension) return { key, scale: null, current: [], under: [] }
      const { width, height } = size()
      const same = prev.key === key
      // The same object stays while its address does, so the lists can
      // change without making its image again.
      const known = new Map<string, Asked>()
      if (same)
        for (const p of [...prev.current, ...prev.under]) known.set(p.url, p)
      const list = visiblePieces(v, width, height, dimension.regions).map(
        (p) => {
          const url = worldMapPieceUrl({
            instanceId: props.instanceId,
            world: props.world,
            dimension: dimension.id,
            scale: p.scale,
            x: p.x,
            z: p.z,
            rev:
              p.scale === 0
                ? regionRevisions().get(`${p.x},${p.z}`)!
                : dimension.revision
          })
          return known.get(url) ?? { url, scale: p.scale, x: p.x, z: p.z }
        }
      )
      const scale = scaleFor(v.zoom)
      let under: Asked[] = []
      if (same && prev.scale !== null) {
        if (prev.scale === scale) under = prev.under
        else {
          const loaded = prev.current.filter((p) => state[p.url] === "loaded")
          // Crossing again before anything of the last scale loaded keeps
          // what was beneath it: never more than two scales at once.
          under = loaded.length > 0 ? loaded : prev.under
        }
      }
      under = under.filter((old) => {
        if (old.scale === scale) return false
        const box = pieceBox(v, old, width, height)
        if (
          box.left + box.width <= 0 ||
          box.left >= width ||
          box.top + box.height <= 0 ||
          box.top >= height
        )
          return false
        const covering = list.filter((p) => overlaps(p, old))
        return covering.some((p) => !state[p.url])
      })
      return {
        key,
        scale,
        // An empty or failed piece is not asked again under the same address.
        current: list.filter((p) => state[p.url] !== "failed"),
        under
      }
    },
    { key: "", scale: null, current: [], under: [] }
  )

  // A piece keeps its slot while it loads and is on screen; a free slot goes
  // to the waiting piece nearest the centre of the view as it is now.
  const loading = createMemo<Set<string>>(
    (prev) => {
      const [state] = settled()
      const current = layout().current
      const onScreen = new Set(current.map((p) => p.url))
      const next = new Set<string>()
      for (const url of prev)
        if (onScreen.has(url) && !state[url]) next.add(url)
      for (const p of current) {
        if (next.size >= MAX_LOADING) break
        if (!state[p.url]) next.add(p.url)
      }
      return next
    },
    new Set(),
    { equals: sameSet }
  )
  const waiting = createMemo(() => {
    const [state] = settled()
    return layout().current.filter((p) => !state[p.url])
  })
  const drawn = createMemo(() => [...layout().under, ...layout().current])

  const boxStyle = (piece: Asked) => {
    const v = view()
    if (!v) return {}
    const box = pieceBox(v, piece, size().width, size().height)
    return {
      left: `${box.left}px`,
      top: `${box.top}px`,
      width: `${box.width}px`,
      height: `${box.height}px`
    }
  }
  const layerStyle = () => {
    const v = view()
    if (!v) return { display: "none" } satisfies JSX.CSSProperties
    const piecePixel = v.zoom * 2 ** scaleFor(v.zoom)
    return {
      // Blocks stay crisp when enlarged, and smooth when shrunk.
      "image-rendering": piecePixel >= 1 ? "pixelated" : "auto"
    } satisfies JSX.CSSProperties
  }

  const local = (e: MouseEvent) => {
    const rect = root.getBoundingClientRect()
    return { x: e.clientX - rect.left, y: e.clientY - rect.top }
  }

  let press: {
    id: number
    x: number
    y: number
    start: View
    moving: boolean
  } | null = null
  let lastClient = { x: 0, y: 0 }
  const [dragging, setDragging] = createSignal(false)

  // Drags and zooms are gathered and applied once per animation frame.
  let frame: number | undefined
  let draggedTo: View | null = null
  let zoomFactor = 1
  let zoomAt = { x: 0, y: 0 }
  const flush = () => {
    frame = undefined
    let v = draggedTo ?? view()
    draggedTo = null
    if (!v) return
    if (zoomFactor !== 1) {
      const { width, height } = size()
      v = zoomAround(v, zoomFactor, zoomAt.x, zoomAt.y, width, height, limits())
      zoomFactor = 1
      // A drag goes on from the zoomed view.
      if (press) press = { ...press, ...lastClient, start: v }
    }
    setView(v)
  }
  const schedule = () => {
    frame ??= requestAnimationFrame(flush)
  }
  const zoomBy = (factor: number, at: { x: number; y: number }) => {
    zoomFactor *= factor
    zoomAt = at
    schedule()
  }

  const endDrag = () => {
    if (!press) return
    if (root.hasPointerCapture(press.id)) root.releasePointerCapture(press.id)
    press = null
    setDragging(false)
  }
  const onPointerDown = (e: PointerEvent) => {
    const v = view()
    if (e.button !== 0 || !v) return
    root.setPointerCapture(e.pointerId)
    lastClient = { x: e.clientX, y: e.clientY }
    press = {
      id: e.pointerId,
      x: e.clientX,
      y: e.clientY,
      start: v,
      moving: false
    }
  }
  const onPointerMove = (e: PointerEvent) => {
    setPointerAt(local(e))
    if (press?.id !== e.pointerId) return
    // The release happened where this element could not hear it.
    if (e.buttons === 0) return endDrag()
    lastClient = { x: e.clientX, y: e.clientY }
    const dx = e.clientX - press.x
    const dy = e.clientY - press.y
    if (!press.moving && Math.hypot(dx, dy) < DRAG_THRESHOLD) return
    press.moving = true
    setDragging(true)
    draggedTo = dragged(press.start, dx, dy)
    schedule()
  }
  const onPointerEnd = (e: PointerEvent) => {
    if (press?.id === e.pointerId) endDrag()
  }

  const onWheel = (e: WheelEvent) => {
    e.preventDefault()
    const at = local(e)
    setPointerAt(at)
    const pixels =
      e.deltaY *
      (e.deltaMode === WheelEvent.DOM_DELTA_LINE
        ? WHEEL_LINE_PIXELS
        : e.deltaMode === WheelEvent.DOM_DELTA_PAGE
          ? WHEEL_PAGE_PIXELS
          : 1)
    zoomBy(WHEEL_FACTOR ** (-pixels / WHEEL_NOTCH), at)
  }

  onMount(() => {
    // Not passive: the wheel zooms the map instead of scrolling the page.
    root.addEventListener("wheel", onWheel, { passive: false })
    onCleanup(() => root.removeEventListener("wheel", onWheel))
    onCleanup(() => {
      if (frame !== undefined) cancelAnimationFrame(frame)
    })
    setSize({ width: root.clientWidth, height: root.clientHeight })
    if (typeof ResizeObserver === "undefined") return
    const observer = new ResizeObserver((entries) => {
      const box = entries[entries.length - 1].contentRect
      setSize({ width: box.width, height: box.height })
    })
    observer.observe(root)
    onCleanup(() => observer.disconnect())
  })

  const coordinates = () => {
    const at = pointerAt()
    const v = view()
    if (!at || !v) return null
    return blockAt(v, at.x, at.y, size().width, size().height)
  }

  const dimensionName = (id: string) =>
    DIMENSION_NAMES[id] ? t(DIMENSION_NAMES[id]) : id

  /** Controls over the map take their own presses. */
  const keepPress = (e: Event) => e.stopPropagation()

  const Marker = (marker: {
    point: FEMapPoint | null
    icon: string
    label: string
    testId: string
    pin?: boolean
  }) => (
    <Show
      when={
        view() && marker.point?.dimension === props.dimension
          ? marker.point
          : null
      }
    >
      {(point) => {
        const at = () =>
          screenOf(
            view()!,
            point().x + 0.5,
            point().z + 0.5,
            size().width,
            size().height
          )
        return (
          <Tooltip>
            <TooltipTrigger
              as="div"
              data-testid={marker.testId}
              class={`${marker.icon} text-lightSlate-50 absolute h-5 w-5 drop-shadow-[0_1px_2px_rgba(0,0,0,0.9)]`}
              style={{
                left: `${at().left}px`,
                top: `${at().top}px`,
                // A pin points at its block with its tip.
                transform: marker.pin
                  ? "translate(-50%, -100%)"
                  : "translate(-50%, -50%)"
              }}
            />
            <TooltipContent>{marker.label}</TooltipContent>
          </Tooltip>
        )
      }}
    </Show>
  )

  return (
    <div
      ref={root}
      data-testid="world-map"
      class="bg-darkSlate-900 relative h-full w-full select-none overflow-hidden"
      classList={{
        "cursor-grab": !dragging(),
        "cursor-grabbing": dragging()
      }}
      style={{ "touch-action": "none" }}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerEnd}
      onPointerCancel={onPointerEnd}
      onLostPointerCapture={onPointerEnd}
      onPointerLeave={() => setPointerAt(null)}
      onDblClick={(e) => zoomBy(2, local(e))}
    >
      <style>{SHEEN_STYLE}</style>
      <div class="world-map-layer absolute inset-0" style={layerStyle()}>
        {/* Placeholders lie lowest, so the pieces of the last scale stay
            seen above them until the new pieces cover them. */}
        <For each={waiting()}>
          {(piece) => (
            <div
              data-testid="world-map-loading"
              class="world-map-loading absolute"
              style={boxStyle(piece)}
            >
              <div class="world-map-sheen" />
            </div>
          )}
        </For>
        <For each={drawn()}>
          {(piece) => {
            const [state, setState] = untrack(settled)
            return (
              <Show
                when={state[piece.url] === "loaded" || loading().has(piece.url)}
              >
                <PieceImage
                  url={piece.url}
                  style={boxStyle(piece)}
                  onSettled={(answer) => setState(piece.url, answer)}
                />
              </Show>
            )
          }}
        </For>
      </div>
      <Marker
        point={props.map.spawn}
        icon="i-hugeicons:home-01"
        label={t("content:_trn_worlds.map.spawn")}
        testId="world-map-spawn"
      />
      <Marker
        point={props.map.player}
        icon="i-hugeicons:location-01"
        label={t("content:_trn_worlds.map.player")}
        testId="world-map-player"
        pin
      />
      <Show when={coordinates()}>
        {(block) => (
          <div
            data-testid="world-map-coordinates"
            class="text-lightSlate-300 bg-darkSlate-800/80 pointer-events-none absolute bottom-2 left-2 rounded px-1.5 py-0.5 text-xs tabular-nums"
          >
            {`${block().x}, ${block().z}`}
          </div>
        )}
      </Show>
      <Show when={props.map.dimensions.length > 1}>
        <div
          data-testid="world-map-dimension"
          class="absolute left-2 top-2 w-44"
          onPointerDown={keepPress}
          onDblClick={keepPress}
        >
          <Select
            value={props.dimension}
            options={props.map.dimensions.map((d) => d.id)}
            onChange={(id) => {
              if (id) props.onDimension(id)
            }}
            itemComponent={(item) => (
              <SelectItem item={item.item}>
                {dimensionName(item.item.rawValue)}
              </SelectItem>
            )}
          >
            <SelectTrigger class="h-8 text-xs">
              <SelectValue<string>>
                {(state) => dimensionName(state.selectedOption())}
              </SelectValue>
            </SelectTrigger>
            <SelectContent />
          </Select>
        </div>
      </Show>
      <div
        class="bg-darkSlate-800/80 absolute right-2 top-2 flex rounded p-1"
        onPointerDown={keepPress}
        onDblClick={keepPress}
      >
        <IconAction
          icon={
            props.filled
              ? "i-hugeicons:minimize-screen"
              : "i-hugeicons:maximize-screen"
          }
          label={t(
            props.filled
              ? "content:_trn_worlds.map.shrink"
              : "content:_trn_worlds.map.fill"
          )}
          onClick={() => props.onFill(!props.filled)}
          testId="world-map-fill"
        />
      </div>
    </div>
  )
}

export default WorldMap
