import type { FETask, FEThemeSounds } from "@gd/core_module/bindings"
import { fileName } from "./background"
import type { MotionState } from "./motion"

export const SOUND_EVENTS = [
  "launch",
  "downloadComplete",
  "click",
  "notification",
  "error"
] as const
export type SoundEvent = (typeof SOUND_EVENTS)[number]

/** Fixed and low: a theme picks the files, never the level. */
export const SOUND_VOLUME = 0.3
/** Every sound is cut here, however long its file is. */
export const SOUND_MAX_MS = 4000
/** The least time between two plays of the same event. */
export const SOUND_GAP_MS: Record<SoundEvent, number> = {
  launch: 2000,
  downloadComplete: 1000,
  click: 120,
  notification: 500,
  error: 500
}

/** Tasks that download something: leaving the list without failing is a finished download. */
export const DOWNLOAD_TASKS: readonly string[] = [
  "InstanceTaskPrepare",
  "InstanceTaskInstallMod",
  "ServerTaskInstall",
  "ServerTaskInstallFromModpack",
  "ServerTaskInstallMod"
]

/** A failed sound file is re-created after this delay... */
export const SOUND_RELOAD_MS = 1000
/** ...at most this many times per URL for the session. */
export const MAX_SOUND_RELOADS = 3

export function shouldPlay(s: {
  enabled: boolean
  motion: MotionState
  hasSound: boolean
  lastMs: number | undefined
  nowMs: number
  event: SoundEvent
}): boolean {
  if (!s.enabled || !s.hasSound || s.motion !== "run") return false
  return s.lastMs === undefined || s.nowMs - s.lastMs >= SOUND_GAP_MS[s.event]
}

/** Whether any event has a file. */
export function hasSounds(sounds: FEThemeSounds | null | undefined): boolean {
  return SOUND_EVENTS.some((event) => sounds?.[event] != null)
}

/** Same URLs for every event (a refetched or re-applied theme is a new object). */
export function sameSounds(
  a: FEThemeSounds | null,
  b: FEThemeSounds | null
): boolean {
  if (!a || !b) return a === b
  return SOUND_EVENTS.every((event) => a[event] === b[event])
}

const taskIds = (tasks: FETask[]) => new Set(tasks.map((t) => t.id))

/** A launch task appeared. False on the first list (`prev` undefined). */
export function launchesStarted(
  prev: FETask[] | undefined,
  next: FETask[] | undefined
): boolean {
  if (!prev || !next) return false
  const before = taskIds(prev)
  return next.some(
    (t) => t.name.translation === "InstanceTaskLaunch" && !before.has(t.id)
  )
}

/** A download task left the list without having failed. False on the first list. */
export function downloadsFinished(
  prev: FETask[] | undefined,
  next: FETask[] | undefined
): boolean {
  if (!prev || !next) return false
  const after = taskIds(next)
  return prev.some(
    (t) =>
      DOWNLOAD_TASKS.includes(t.name.translation) &&
      t.progress.type !== "Failed" &&
      !after.has(t.id)
  )
}

/**
 * The sound of a new toast, from somoto's `data-type`. A plain `toast()` has
 * no type (null), and is a notification like any other.
 */
export function toastSound(
  type: string | null
): "notification" | "error" | null {
  if (type === "loading") return null
  return type === "error" ? "error" : "notification"
}

const PRIMARY = '[data-gdl~="button.primary"]'
const DISABLED = '[data-gdl~="button.disabled"]'

/** The click landed on (or inside) a primary button that isn't disabled. */
export function isPrimaryButton(target: EventTarget | null): boolean {
  if (typeof Node === "undefined" || !(target instanceof Node)) return false
  const el = target instanceof Element ? target : target.parentElement
  const button = el?.closest(PRIMARY)
  return !!button && !button.matches(DISABLED)
}

const TOAST = "[data-somoto-toast]"

/**
 * Reports each toast added under `region` once, with its type. somoto sets
 * its attributes reactively, so they are read in a microtask. somoto keys its
 * list by object: a dismiss swaps the toast for a copy marked
 * `data-removed="true"` (a fresh node), and an update by id is a fresh node
 * too. So a node counts once it is still in place and not marked removed.
 * Returns the disconnect; nothing is reported after it.
 */
export function watchToasts(
  region: Element,
  onToast: (type: string | null) => void
): () => void {
  let live = true
  // a toast moved within the list is the same toast
  const seen = new WeakSet<Element>()
  const report = (toast: Element) => {
    if (seen.has(toast)) return
    seen.add(toast)
    queueMicrotask(() => {
      if (!live || !toast.isConnected) return
      if (toast.getAttribute("data-removed") === "true") return
      onToast(toast.getAttribute("data-type"))
    })
  }
  const observer = new MutationObserver((records) => {
    for (const record of records) {
      for (const node of Array.from(record.addedNodes)) {
        if (!(node instanceof Element)) continue
        if (node.matches(TOAST)) report(node)
        node.querySelectorAll(TOAST).forEach(report)
      }
    }
  })
  observer.observe(region, { childList: true, subtree: true })
  return () => {
    live = false
    observer.disconnect()
  }
}

/** The part of an HTMLAudioElement a sound bank uses. */
export interface SoundAudio {
  src: string
  preload: string
  volume: number
  currentTime: number
  readonly paused: boolean
  play(): Promise<void>
  pause(): void
  load(): void
  removeAttribute(name: string): void
  addEventListener(type: string, listener: () => void): void
  removeEventListener(type: string, listener: () => void): void
}

/** What outlives one bank, per URL: the reloads used and a reported play refusal. */
export interface SoundSession {
  reloads: Map<string, number>
  warned: Set<string>
}

export const createSoundSession = (): SoundSession => ({
  reloads: new Map(),
  warned: new Set()
})

export interface SoundBankDeps {
  createAudio(): SoundAudio
  setTimeout(cb: () => void, ms: number): unknown
  clearTimeout(id: unknown): void
  now(): number
  session: SoundSession
}

export interface SoundBank {
  /** Plays the event's sound if the rules allow it; true when it started. */
  play(event: SoundEvent): boolean
  /** Follows motion(): leaving run pauses what plays and drops every timer. */
  setMotion(state: MotionState): void
  /** Pauses and releases every player and drops every timer. */
  dispose(): void
}

interface Player {
  url: string
  el: SoundAudio
  /** The SOUND_MAX_MS cut, while the sound plays. */
  cut: unknown
  /** The file failed: silent until the player is re-created. */
  broken: boolean
  detach(): void
}

/**
 * One preloading player per sound of a theme; a bank exists only while sounds
 * are switched on. Sounds play only under run, each is cut at SOUND_MAX_MS,
 * and no timer runs while motion isn't run. A player whose file fails (the
 * core cuts a theme's streams before replacing it) is re-created once after
 * SOUND_RELOAD_MS; a good load re-arms that, up to MAX_SOUND_RELOADS per URL
 * for the session.
 */
export function createSoundBank(
  sounds: FEThemeSounds,
  motion: MotionState,
  deps: SoundBankDeps
): SoundBank {
  const { session } = deps
  const players = new Map<SoundEvent, Player>()
  const last = new Map<SoundEvent, number>()
  // Players re-created since their last good load: one reload per failure.
  // By event, so two events sharing a file both recover; the session budget
  // stays per URL.
  const retried = new Set<SoundEvent>()
  // reloads owed; their timers run only under run
  const owed = new Set<SoundEvent>()
  const reloadTimers = new Map<SoundEvent, unknown>()
  let state = motion
  let disposed = false

  const clearCut = (p: Player) => {
    if (p.cut === null) return
    deps.clearTimeout(p.cut)
    p.cut = null
  }

  const stop = (p: Player) => {
    clearCut(p)
    if (!p.el.paused) p.el.pause()
  }

  const release = (p: Player) => {
    stop(p)
    p.detach()
    // dropping the source and loading aborts the download
    p.el.removeAttribute("src")
    p.el.load()
  }

  const clearReloadTimers = () => {
    for (const id of reloadTimers.values()) deps.clearTimeout(id)
    reloadTimers.clear()
  }

  const scheduleReload = (event: SoundEvent) => {
    if (state !== "run" || reloadTimers.has(event)) return
    const id = deps.setTimeout(() => {
      reloadTimers.delete(event)
      owed.delete(event)
      const old = players.get(event)
      if (!old) return
      release(old)
      players.set(event, create(event, old.url))
    }, SOUND_RELOAD_MS)
    reloadTimers.set(event, id)
  }

  const onError = (event: SoundEvent, p: Player) => {
    console.warn(`Theme sound failed to load: ${fileName(p.url)}`)
    clearCut(p)
    p.broken = true
    if (retried.has(event)) return
    const used = session.reloads.get(p.url) ?? 0
    if (used >= MAX_SOUND_RELOADS) return
    session.reloads.set(p.url, used + 1)
    retried.add(event)
    owed.add(event)
    scheduleReload(event)
  }

  function create(event: SoundEvent, url: string): Player {
    const el = deps.createAudio()
    const p: Player = { url, el, cut: null, broken: false, detach: () => {} }
    const onFail = () => onError(event, p)
    const onEnded = () => clearCut(p)
    const onLoaded = () => retried.delete(event)
    el.addEventListener("error", onFail)
    el.addEventListener("ended", onEnded)
    el.addEventListener("loadeddata", onLoaded)
    p.detach = () => {
      el.removeEventListener("error", onFail)
      el.removeEventListener("ended", onEnded)
      el.removeEventListener("loadeddata", onLoaded)
    }
    el.preload = "auto"
    el.volume = SOUND_VOLUME
    el.src = url
    return p
  }

  for (const event of SOUND_EVENTS) {
    const url = sounds[event]
    if (url) players.set(event, create(event, url))
  }

  return {
    play(event) {
      const p = players.get(event)
      const nowMs = deps.now()
      const ok = shouldPlay({
        enabled: !disposed,
        motion: state,
        hasSound: p !== undefined && !p.broken,
        lastMs: last.get(event),
        nowMs,
        event
      })
      if (!ok || !p) return false
      last.set(event, nowMs)
      // A button's click comes right before what it starts (a launch, a
      // toast): the click gives way rather than overlap.
      if (event !== "click") {
        const click = players.get("click")
        if (click) stop(click)
      }
      p.el.currentTime = 0
      p.el.play().catch((e: unknown) => {
        // pause() cutting a pending play() is not a failure
        if ((e as { name?: string } | null)?.name === "AbortError") return
        if (session.warned.has(p.url)) return
        session.warned.add(p.url)
        console.warn(`Theme sound could not play: ${fileName(p.url)}`, e)
      })
      clearCut(p)
      p.cut = deps.setTimeout(() => {
        p.cut = null
        p.el.pause()
      }, SOUND_MAX_MS)
      return true
    },
    setMotion(next) {
      if (disposed) return
      state = next
      if (next === "run") {
        for (const event of owed) scheduleReload(event)
        return
      }
      for (const p of players.values()) stop(p)
      clearReloadTimers()
    },
    dispose() {
      if (disposed) return
      disposed = true
      for (const p of players.values()) release(p)
      players.clear()
      clearReloadTimers()
      owed.clear()
    }
  }
}
