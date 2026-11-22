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
  createEffect,
  createMemo,
  createSignal,
  For,
  type JSX,
  on,
  onCleanup,
  onMount,
  Show,
  untrack
} from "solid-js"
import { createStore, reconcile } from "solid-js/store"
import { worldMapPieceUrl } from "@/utils/worldMapUrl"
import IconAction from "./IconAction"
import {
  blockAt,
  dragged,
  fitAll,
  MAX_ZOOM,
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
  map: FEWorldMap
  dimension: string
  onDimension: (_id: string) => void
  filled: boolean
  onFill: (_filled: boolean) => void
}

const WHEEL_FACTOR = 1.25
/** Trackpads send a stream of small deltas: one zoom step per this long. */
const WHEEL_STEP_MS = 40
/** A press that moves less than this is a click, not a drag. */
const DRAG_THRESHOLD = 3

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

const SHEEN_STYLE = `
.world-map-loading {
  background-color: rgb(var(--darkSlate-900));
  background-image: linear-gradient(100deg, transparent 35%, rgb(var(--darkSlate-700) / 0.6) 50%, transparent 65%);
  background-size: 250% 100%;
  animation-name: world-map-sheen;
  animation-duration: 1.8s;
  animation-timing-function: ease-in-out;
  animation-iteration-count: infinite;
}
@keyframes world-map-sheen {
  from { background-position: 100% 0; }
  to { background-position: 0 0; }
}
.potato-pc .world-map-loading {
  animation-name: none;
  background-image: none;
  background-color: rgb(var(--darkSlate-800));
}
@media (prefers-reduced-motion: reduce) {
  .world-map-loading {
    animation-name: none;
    background-image: none;
    background-color: rgb(var(--darkSlate-800));
  }
}
`

const WorldMap = (props: WorldMapProps) => {
  const [t] = useTransContext()
  let root!: HTMLDivElement

  const [size, setSize] = createSignal(
    { width: 0, height: 0 },
    { equals: (a, b) => a.width === b.width && a.height === b.height }
  )
  const [view, setView] = createSignal<View | null>(null)
  // Pieces sit in the layer in blocks from this point, so their CSS
  // positions stay small however far out in the world the map is.
  const [anchor, setAnchor] = createSignal({ x: 0, z: 0 })
  const [settled, setSettled] = createStore<
    Record<string, "loaded" | "failed">
  >({})
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

  const firstView = (): View => {
    const inShown = (p: FEMapPoint | null) =>
      p?.dimension === props.dimension ? p : null
    const start = inShown(props.map.player) ?? inShown(props.map.spawn)
    if (start)
      return { centerX: start.x + 0.5, centerZ: start.z + 0.5, zoom: 1 }
    return fitAll(shown()?.regions ?? [], size().width, size().height)
  }

  createEffect(
    on(
      [() => props.world, () => props.dimension],
      () => {
        setView(null)
        setSettled(reconcile({}))
      },
      { defer: true }
    )
  )
  createEffect(() => {
    const { width, height } = size()
    if (view() !== null || width <= 0 || height <= 0) return
    const first = untrack(firstView)
    setAnchor({ x: Math.floor(first.centerX), z: Math.floor(first.centerZ) })
    setView(first)
  })

  let asked = new Map<string, Asked>()
  const pieces = createMemo(() => {
    const v = view()
    const dimension = shown()
    if (!v || !dimension) {
      asked = new Map()
      return []
    }
    const next = new Map<string, Asked>()
    const list = visiblePieces(
      v,
      size().width,
      size().height,
      dimension.regions
    ).map((p) => {
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
      const piece = asked.get(url) ?? { url, scale: p.scale, x: p.x, z: p.z }
      next.set(url, piece)
      return piece
    })
    asked = next
    // An empty or failed piece is not asked again under the same address.
    return list.filter((p) => settled[p.url] !== "failed")
  })

  const layerStyle = () => {
    const v = view()
    if (!v) return { display: "none" } satisfies JSX.CSSProperties
    const { width, height } = size()
    const a = anchor()
    const piecePixel = v.zoom * 2 ** scaleFor(v.zoom)
    return {
      transform: `translate(${width / 2 - (v.centerX - a.x) * v.zoom}px, ${
        height / 2 - (v.centerZ - a.z) * v.zoom
      }px) scale(${v.zoom})`,
      "transform-origin": "0 0",
      // Blocks stay crisp when enlarged, and smooth when shrunk.
      "image-rendering": piecePixel >= 1 ? "pixelated" : "auto"
    } satisfies JSX.CSSProperties
  }

  const local = (e: MouseEvent) => {
    const rect = root.getBoundingClientRect()
    return { x: e.clientX - rect.left, y: e.clientY - rect.top }
  }
  const zoomAt = (factor: number, at: { x: number; y: number }) => {
    const v = view()
    if (!v) return
    const { width, height } = size()
    setView(zoomAround(v, factor, at.x, at.y, width, height, limits()))
  }

  let press: {
    id: number
    x: number
    y: number
    start: View
    moving: boolean
  } | null = null
  const [dragging, setDragging] = createSignal(false)
  const onPointerDown = (e: PointerEvent) => {
    const v = view()
    if (e.button !== 0 || !v) return
    root.setPointerCapture(e.pointerId)
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
    const dx = e.clientX - press.x
    const dy = e.clientY - press.y
    if (!press.moving && Math.hypot(dx, dy) < DRAG_THRESHOLD) return
    press.moving = true
    setDragging(true)
    setView(dragged(press.start, dx, dy))
  }
  const onPointerEnd = (e: PointerEvent) => {
    if (press?.id !== e.pointerId) return
    if (root.hasPointerCapture(e.pointerId))
      root.releasePointerCapture(e.pointerId)
    press = null
    setDragging(false)
  }

  let wheelDelta = 0
  let lastWheelStep = -Infinity
  const onWheel = (e: WheelEvent) => {
    e.preventDefault()
    const at = local(e)
    setPointerAt(at)
    wheelDelta += e.deltaY
    const now = Date.now()
    if (now - lastWheelStep < WHEEL_STEP_MS || wheelDelta === 0) return
    const factor = wheelDelta < 0 ? WHEEL_FACTOR : 1 / WHEEL_FACTOR
    wheelDelta = 0
    lastWheelStep = now
    zoomAt(factor, at)
  }

  onMount(() => {
    // Not passive: the wheel zooms the map instead of scrolling the page.
    root.addEventListener("wheel", onWheel, { passive: false })
    onCleanup(() => root.removeEventListener("wheel", onWheel))
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
      onPointerLeave={() => setPointerAt(null)}
      onDblClick={(e) => zoomAt(2, local(e))}
    >
      <style>{SHEEN_STYLE}</style>
      <div class="world-map-layer absolute left-0 top-0" style={layerStyle()}>
        <For each={pieces()}>
          {(piece) => {
            const blocks = PIECE_PIXELS * 2 ** piece.scale
            const box = () => ({
              left: `${piece.x * blocks - anchor().x}px`,
              top: `${piece.z * blocks - anchor().z}px`,
              width: `${blocks}px`,
              height: `${blocks}px`
            })
            return (
              <>
                <img
                  data-testid="world-map-piece"
                  src={piece.url}
                  alt=""
                  draggable={false}
                  class="absolute block"
                  style={{ ...box(), "max-width": "none" }}
                  onLoad={() => setSettled(piece.url, "loaded")}
                  onError={() => setSettled(piece.url, "failed")}
                />
                <Show when={!settled[piece.url]}>
                  <div
                    data-testid="world-map-loading"
                    class="world-map-loading absolute"
                    style={box()}
                  />
                </Show>
              </>
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
