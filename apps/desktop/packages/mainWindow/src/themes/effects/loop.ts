export interface Effect {
  readonly fps: number
  resize(width: number, height: number): void
  frame(timeMs: number, dtSeconds: number): void
}

export interface LoopDeps {
  raf(cb: (t: number) => void): number
  caf(id: number): void
  setTimeout(cb: () => void, ms: number): number
  clearTimeout(id: number): void
  /** The clock the timer waits are measured on, in the same ms as raf time. */
  now(): number
}

/** The browser's frame and timer functions. */
export const browserLoopDeps: LoopDeps = {
  raf: (cb) => requestAnimationFrame(cb),
  caf: (id) => cancelAnimationFrame(id),
  setTimeout: (cb, ms) => window.setTimeout(cb, ms),
  clearTimeout: (id) => window.clearTimeout(id),
  now: () => performance.now()
}

interface Entry {
  effect: Effect
  // null until the first frame after (re)starting, so that frame gets dt = 0
  last: number | null
}

// A frame that lands a hair early (rAF jitter) still counts for its slot.
const SLACK_MS = 1
// The next effect is due within about a frame: request the frame right away.
const FRAME_MS = 8
// The timer fires this much before the due time, so one that runs a little
// late still requests the frame in time for the refresh the effect is due on.
const TIMER_LEAD_MS = 2

/**
 * One requestAnimationFrame chain for every effect, paced to the effects'
 * own frame rates: between due frames it waits on a timer instead of waking
 * on every display refresh. While stopped nothing is scheduled: the pending
 * timer and raf are cancelled, not skipped.
 */
export function createEffectsLoop(deps: LoopDeps) {
  const entries = new Set<Entry>()
  let running = false
  let handle: number | null = null
  let timer: number | null = null

  const requestFrame = () => {
    if (handle === null) handle = deps.raf(tick)
  }

  const cancelTimer = () => {
    if (timer !== null) deps.clearTimeout(timer)
    timer = null
  }

  // When the earliest effect is next due, in raf time; Infinity with none.
  const nextDue = () => {
    let due = Infinity
    for (const entry of entries) {
      const at =
        entry.last === null
          ? -Infinity
          : entry.last + 1000 / entry.effect.fps - SLACK_MS
      if (at < due) due = at
    }
    return due
  }

  const tick = (t: number) => {
    handle = null
    if (!running) return
    for (const entry of entries) {
      if (entry.last === null) {
        entry.last = t
        entry.effect.frame(t, 0)
        continue
      }
      const elapsed = t - entry.last
      if (elapsed >= 1000 / entry.effect.fps - SLACK_MS) {
        entry.last = t
        entry.effect.frame(t, Math.min(0.05, elapsed / 1000))
      }
    }
    // from now, not from the frame time: slow frames must not push the next
    // timer late and drag the effects below their rate
    const wait = nextDue() - deps.now()
    // Nothing to draw: add() starts the chain again.
    if (wait === Infinity) return
    if (wait > FRAME_MS) {
      timer = deps.setTimeout(() => {
        timer = null
        if (running) requestFrame()
      }, wait - TIMER_LEAD_MS)
    } else {
      requestFrame()
    }
  }

  return {
    add(effect: Effect): () => void {
      const entry: Entry = { effect, last: null }
      entries.add(entry)
      // A new effect draws on the next refresh, not after the current wait.
      if (running) {
        cancelTimer()
        requestFrame()
      }
      return () => {
        entries.delete(entry)
      }
    },
    setRunning(next: boolean): void {
      if (next === running) return
      running = next
      if (running) {
        for (const entry of entries) entry.last = null
        requestFrame()
      } else {
        cancelTimer()
        if (handle !== null) deps.caf(handle)
        handle = null
      }
    },
    isScheduled(): boolean {
      return handle !== null || timer !== null
    }
  }
}
