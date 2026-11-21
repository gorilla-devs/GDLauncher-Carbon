import type { FEThemeBackground } from "@gd/core_module/bindings"
import type { MotionState } from "./motion"

export const BACKGROUND_DEFAULTS = {
  fit: "cover",
  position: "center",
  dim: 0,
  blur: 0
} as const

interface Layer {
  src: string
  style: Record<string, string>
}

export interface BackgroundModel {
  media: { kind: "image" | "video"; src: string; style: Record<string, string> }
  motion: Layer[]
  dim: number
  playing: boolean
  rewind: boolean
}

export type VideoAction = "play" | "pause" | "rewind" | "release" | "none"

// The file name of a theme asset URL: an installed theme's file is in the
// core URL's `path` parameter. Never an absolute path, never the token.
export const fileName = (src: string) => {
  let file = src
  try {
    const url = new URL(src)
    file = url.searchParams.get("path") ?? url.pathname
  } catch {
    // not a URL: a plain path
  }
  return file.split(/[\\/]/).pop()?.split("?")[0] ?? ""
}

/**
 * An error handler for a background element. It names the file from the
 * element itself, so it never reads a reactive accessor that may be stale by
 * the time the event fires (a theme switch while the file fails).
 */
export function warnOnLoadError(what: string) {
  return (e: Event) => {
    const src = (e.currentTarget as { src?: string } | null)?.src ?? ""
    console.warn(`Theme background ${what} failed to load: ${fileName(src)}`)
  }
}

export const VIDEO_RETRY_MS = 1000
export const MAX_VIDEO_RETRIES = 3

interface VideoEl {
  src?: string
  load?: () => void
}

/**
 * The video's error handling: warns as warnOnLoadError does, and reloads the
 * element once per URL after a short delay. A cut stream (a failed theme
 * replace) leaves the same URL, and Solid skips an unchanged `src`, so
 * without this the video would stay frozen for the session. `load()` leaves
 * the element paused, so `onReload` lets the owner put the playback state
 * back. A successful load re-arms the URL (a later failed replace recovers
 * too). A stream that loads its first frame and then fails every time would
 * repeat about once a second, so each URL is also capped at MAX_VIDEO_RETRIES
 * reloads for the session.
 *
 * Like the sound bank's reloads, the delay runs only under `run`: a failure
 * under `pause` or `off` warns at once and owes its reload, and the delay
 * starts when motion is back to `run`. Leaving `run` drops the timer and
 * keeps the reload owed.
 */
export function retryVideoOnce(
  motion: MotionState,
  deps: {
    setTimeout?(cb: () => void, ms: number): unknown
    clearTimeout?(id: unknown): void
    onReload?(el: VideoEl): void
  } = {}
) {
  const set = deps.setTimeout ?? ((cb, ms) => window.setTimeout(cb, ms))
  const clear = deps.clearTimeout ?? ((id) => window.clearTimeout(id as number))
  const warn = warnOnLoadError("video")
  const retried = new Set<string>()
  const used = new Map<string, number>()
  let state = motion
  let disposed = false
  // the reload owed, the latest failure's; its timer runs only under run
  let owed: { el: VideoEl; src: string } | null = null
  let timer: unknown = null

  const dropTimer = () => {
    if (timer !== null) clear(timer)
    timer = null
  }

  const schedule = () => {
    if (state !== "run" || owed === null || timer !== null) return
    const { el, src } = owed
    timer = set(() => {
      timer = null
      owed = null
      // the source changed or was released meanwhile: nothing to retry
      if (el.src !== src) return
      el.load?.()
      deps.onReload?.(el)
    }, VIDEO_RETRY_MS)
  }

  return {
    onError(e: Event) {
      warn(e)
      const el = e.currentTarget as VideoEl | null
      const src = el?.src
      if (disposed || !el || !src || retried.has(src)) return
      if ((used.get(src) ?? 0) >= MAX_VIDEO_RETRIES) return
      used.set(src, (used.get(src) ?? 0) + 1)
      retried.add(src)
      dropTimer()
      owed = { el, src }
      schedule()
    },
    onLoaded() {
      retried.clear()
    },
    /** Follows motion(): the delay runs only under run. */
    setMotion(next: MotionState) {
      if (disposed) return
      state = next
      if (next === "run") schedule()
      else dropTimer()
    },
    dispose() {
      disposed = true
      dropTimer()
      owed = null
    }
  }
}

/** What to do to the video element when the model changes from prev to next. */
export function videoAction(
  prev: BackgroundModel | null,
  next: BackgroundModel | null
): VideoAction {
  if (next?.media.kind !== "video") {
    return prev?.media.kind === "video" ? "release" : "none"
  }
  if (next.playing) return "play"
  return next.rewind ? "rewind" : "pause"
}

export function backgroundModel(
  bg: FEThemeBackground | null,
  state: MotionState
): BackgroundModel | null {
  if (!bg) return null
  const fit = bg.fit ?? BACKGROUND_DEFAULTS.fit
  const position = bg.position ?? BACKGROUND_DEFAULTS.position
  const dim = bg.dim ?? BACKGROUND_DEFAULTS.dim
  const blur = bg.blur ?? BACKGROUND_DEFAULTS.blur

  const base = (): Record<string, string> => ({
    "object-fit": fit,
    "object-position": position,
    inset: "0",
    "will-change": "transform"
  })

  // Blur only the static layer: re-blurring an animated full-window layer on
  // every frame is too costly. The media is oversized so blurred edges don't fade.
  const mediaStyle = base()
  if (blur > 0) {
    mediaStyle.filter = `blur(${blur}px)`
    // inset plus width/height would be over-constrained, so size it explicitly
    delete mediaStyle.inset
    mediaStyle.top = `${-2 * blur}px`
    mediaStyle.left = `${-2 * blur}px`
    mediaStyle.width = `calc(100% + ${4 * blur}px)`
    mediaStyle.height = `calc(100% + ${4 * blur}px)`
  }

  return {
    media: {
      kind: bg.video ? "video" : "image",
      src: bg.src,
      style: mediaStyle
    },
    motion:
      state === "run" ? bg.motion.map((src) => ({ src, style: base() })) : [],
    dim,
    playing: bg.video && state === "run",
    rewind: state === "off"
  }
}
