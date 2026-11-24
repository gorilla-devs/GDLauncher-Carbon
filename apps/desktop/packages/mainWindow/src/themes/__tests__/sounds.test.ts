// @vitest-environment jsdom
import { describe, it, expect, vi, afterEach } from "vitest"
import type {
  FETask,
  FEThemeSounds,
  Progress,
  Translation
} from "@gd/core_module/bindings"
import {
  createSoundBank,
  createSoundSession,
  DOWNLOAD_TASKS,
  downloadsFinished,
  hasSounds,
  isPrimaryButton,
  launchesStarted,
  MAX_SOUND_RELOADS,
  sameSounds,
  shouldPlay,
  SOUND_EVENTS,
  SOUND_GAP_MS,
  SOUND_MAX_MS,
  SOUND_RELOAD_MS,
  SOUND_VOLUME,
  toastSound,
  watchToasts,
  type SoundEvent
} from "../sounds"
import type { MotionState } from "../motion"

afterEach(() => {
  vi.restoreAllMocks()
  document.body.innerHTML = ""
})

const known: Progress = { type: "Known", value: 0.5 }
const failed: Progress = {
  type: "Failed",
  value: { cause: [], backtrace: "" }
}

const task = (
  id: number,
  translation: string,
  progress: Progress = known
): FETask => ({
  id,
  name: { translation, args: { name: "pack" } } as unknown as Translation,
  progress,
  downloaded: "0",
  download_total: "0",
  active_subtasks: []
})

const noSounds = (): FEThemeSounds => ({
  launch: null,
  downloadComplete: null,
  click: null,
  notification: null,
  error: null
})

const url = (file: string) =>
  `http://127.0.0.1:4650/theme/asset?id=demo&path=sounds/${file}&v=1`

class FakeAudio {
  static all: FakeAudio[] = []
  src = ""
  preload = ""
  volume = 1
  currentTime = 7
  paused = true
  plays = 0
  pauses = 0
  loads = 0
  playResult: () => Promise<void> = () => Promise.resolve()
  private listeners = new Map<string, Set<() => void>>()
  constructor() {
    FakeAudio.all.push(this)
  }
  play() {
    this.plays++
    this.paused = false
    return this.playResult()
  }
  pause() {
    this.pauses++
    this.paused = true
  }
  load() {
    this.loads++
  }
  removeAttribute(name: string) {
    if (name === "src") this.src = ""
  }
  addEventListener(type: string, cb: () => void) {
    if (!this.listeners.has(type)) this.listeners.set(type, new Set())
    this.listeners.get(type)!.add(cb)
  }
  removeEventListener(type: string, cb: () => void) {
    this.listeners.get(type)?.delete(cb)
  }
  fire(type: string) {
    if (type === "ended") this.paused = true
    for (const cb of [...(this.listeners.get(type) ?? [])]) cb()
  }
}

// A manual clock: timers run only when the test advances it.
function fakeClock() {
  let now = 1_000
  let nextId = 1
  const timers = new Map<number, { at: number; ms: number; cb: () => void }>()
  return {
    now: () => now,
    setTimeout: (cb: () => void, ms: number) => {
      const id = nextId++
      timers.set(id, { at: now + ms, ms, cb })
      return id
    },
    clearTimeout: (id: unknown) => {
      timers.delete(id as number)
    },
    /** The delays of the timers still waiting. */
    pending: () => [...timers.values()].map((t) => t.ms),
    advance(ms: number) {
      now += ms
      const due = [...timers.entries()]
        .filter(([, t]) => t.at <= now)
        .sort((a, b) => a[1].at - b[1].at)
      for (const [id, t] of due) {
        if (!timers.has(id)) continue
        timers.delete(id)
        t.cb()
      }
    }
  }
}

function bank(
  sounds: Partial<FEThemeSounds>,
  motion: MotionState = "run",
  session = createSoundSession()
) {
  FakeAudio.all = []
  const clock = fakeClock()
  const warn = vi.spyOn(console, "warn").mockImplementation(() => {})
  const error = vi.spyOn(console, "error").mockImplementation(() => {})
  const b = createSoundBank({ ...noSounds(), ...sounds }, motion, {
    createAudio: () => new FakeAudio(),
    setTimeout: clock.setTimeout,
    clearTimeout: clock.clearTimeout,
    now: clock.now,
    session
  })
  const audio = (src: string) =>
    FakeAudio.all.filter((a) => a.src === src).at(-1)!
  return { b, clock, warn, error, audio, session }
}

describe("sound rules", () => {
  it("sounds follow the switch and the motion state", () => {
    const base = {
      enabled: true,
      motion: "run" as MotionState,
      hasSound: true,
      lastMs: undefined,
      nowMs: 5_000,
      event: "click" as SoundEvent
    }
    const table: [Partial<typeof base>, boolean][] = [
      [{ enabled: false }, false],
      [{ motion: "pause" }, false],
      [{ motion: "off" }, false],
      [{ motion: "run" }, true],
      [{ hasSound: false }, false]
    ]
    for (const [patch, expected] of table) {
      expect(shouldPlay({ ...base, ...patch })).toBe(expected)
    }
  })

  it("rapid events are spaced", () => {
    const s = (event: SoundEvent, nowMs: number) =>
      shouldPlay({
        enabled: true,
        motion: "run",
        hasSound: true,
        lastMs: 1000,
        nowMs,
        event
      })
    expect(s("click", 1119)).toBe(false)
    expect(s("click", 1120)).toBe(true)
    expect(s("launch", 2999)).toBe(false)
    expect(s("launch", 3000)).toBe(true)
    expect(SOUND_GAP_MS).toEqual({
      launch: 2000,
      downloadComplete: 1000,
      click: 120,
      notification: 500,
      error: 500
    })
    expect(SOUND_EVENTS).toEqual([
      "launch",
      "downloadComplete",
      "click",
      "notification",
      "error"
    ])
    expect(SOUND_VOLUME).toBe(0.3)
  })

  it("long sounds are cut", () => {
    expect(SOUND_MAX_MS).toBe(4000)
    const { b, clock, audio } = bank({ click: url("long.wav") })
    expect(b.play("click")).toBe(true)
    const a = audio(url("long.wav"))
    expect(a.plays).toBe(1)
    expect(a.currentTime).toBe(0)
    expect(clock.pending()).toEqual([4000])
    clock.advance(3999)
    expect(a.paused).toBe(false)
    clock.advance(1)
    expect(a.paused).toBe(true)
    expect(a.pauses).toBe(1)
    expect(clock.pending()).toEqual([])
  })

  it("a sound that ends clears its cut timer, and a replay replaces it", () => {
    const { b, clock, audio } = bank({ click: url("c.wav") })
    b.play("click")
    clock.advance(200)
    b.play("click")
    expect(clock.pending()).toEqual([4000])
    const a = audio(url("c.wav"))
    a.fire("ended")
    expect(clock.pending()).toEqual([])
    expect(a.pauses).toBe(0)
  })

  it("each sound's cut timer is its own", () => {
    const { b, clock, audio } = bank({
      notification: url("n.wav"),
      error: url("e.wav")
    })
    b.play("notification")
    clock.advance(1000)
    b.play("error")
    expect(clock.pending()).toEqual([4000, 4000])
    clock.advance(3000)
    expect(audio(url("n.wav")).paused).toBe(true)
    expect(audio(url("e.wav")).paused).toBe(false)
  })

  it("ten rapid clicks play once per gap", () => {
    const { b, clock, audio } = bank({ click: url("c.wav") })
    for (let i = 0; i < 10; i++) {
      b.play("click")
      clock.advance(10)
    }
    expect(audio(url("c.wav")).plays).toBe(1)
    clock.advance(20)
    expect(b.play("click")).toBe(true)
    expect(audio(url("c.wav")).plays).toBe(2)
  })

  it("a click still playing stops when another sound starts", () => {
    const { b, clock, audio } = bank({
      click: url("c.wav"),
      launch: url("l.wav"),
      notification: url("n.wav")
    })
    // Play: the button's click, then the launch task ~100 ms later
    b.play("click")
    const click = audio(url("c.wav"))
    clock.advance(100)
    expect(b.play("launch")).toBe(true)
    expect(click.paused).toBe(true)
    expect(click.pauses).toBe(1)
    // the click's cut timer went with it; the launch keeps its own
    expect(clock.pending()).toEqual([4000])
    expect(audio(url("l.wav")).paused).toBe(false)

    // a sound that doesn't start (inside its gap) leaves the click alone
    clock.advance(200)
    b.play("click")
    clock.advance(50)
    expect(b.play("launch")).toBe(false)
    expect(click.paused).toBe(false)
    // a click never stops another sound
    b.play("notification")
    expect(click.paused).toBe(true)
    clock.advance(200)
    b.play("click")
    expect(audio(url("n.wav")).paused).toBe(false)
  })

  it("two events sharing a failed file both recover", () => {
    const shared = url("both.wav")
    const { b, clock, session } = bank({
      launch: shared,
      downloadComplete: shared
    })
    const [launch, done] = FakeAudio.all
    launch.fire("error")
    done.fire("error")
    expect(clock.pending()).toEqual([SOUND_RELOAD_MS, SOUND_RELOAD_MS])
    // the session budget stays per URL
    expect(session.reloads.get(shared)).toBe(2)
    clock.advance(SOUND_RELOAD_MS)
    expect(FakeAudio.all).toHaveLength(4)
    expect(b.play("launch")).toBe(true)
    expect(b.play("downloadComplete")).toBe(true)

    // a good load of one re-arms only that one
    const [, , launch2, done2] = FakeAudio.all
    launch2.fire("loadeddata")
    done2.fire("error")
    launch2.fire("error")
    expect(clock.pending()).toEqual([SOUND_RELOAD_MS])
    expect(session.reloads.get(shared)).toBe(3)
  })

  it("the gap is per event and only sounds with a URL play", () => {
    const { b } = bank({ click: url("c.wav"), error: url("e.wav") })
    expect(b.play("click")).toBe(true)
    expect(b.play("error")).toBe(true)
    expect(b.play("launch")).toBe(false)
    expect(FakeAudio.all).toHaveLength(2)
  })

  it("each sound with a URL gets one preloading player at the fixed volume", () => {
    bank({ launch: url("l.ogg"), downloadComplete: url("d.mp3") })
    expect(FakeAudio.all.map((a) => a.src)).toEqual([
      url("l.ogg"),
      url("d.mp3")
    ])
    for (const a of FakeAudio.all) {
      expect(a.preload).toBe("auto")
      expect(a.volume).toBe(SOUND_VOLUME)
      expect(a.plays).toBe(0)
    }
  })

  it("leaving run stops a playing sound and clears its timer", () => {
    for (const state of ["pause", "off"] as const) {
      const { b, clock, audio } = bank({
        click: url("c.wav"),
        launch: url("l.wav")
      })
      b.play("click")
      const a = audio(url("c.wav"))
      b.setMotion(state)
      expect(a.paused).toBe(true)
      expect(a.pauses).toBe(1)
      // the idle player is left alone
      expect(audio(url("l.wav")).pauses).toBe(0)
      expect(clock.pending()).toEqual([])

      clock.advance(10_000)
      expect(b.play("click")).toBe(false)
      expect(a.plays).toBe(1)
      expect(clock.pending()).toEqual([])

      b.setMotion("run")
      expect(b.play("click")).toBe(true)
      expect(a.plays).toBe(2)
    }
  })

  it("a bank made under pause stays silent until run", () => {
    const { b, clock, audio } = bank({ click: url("c.wav") }, "pause")
    expect(b.play("click")).toBe(false)
    expect(clock.pending()).toEqual([])
    b.setMotion("run")
    expect(b.play("click")).toBe(true)
    expect(audio(url("c.wav")).plays).toBe(1)
  })

  it("a refused play warns once per URL; an interrupted one stays quiet", async () => {
    const { b, clock, audio, warn, error } = bank({
      click: url("c.wav"),
      error: url("e.wav")
    })
    const flush = async () => {
      for (let i = 0; i < 3; i++) await Promise.resolve()
    }
    const reject = (name: string) => () =>
      Promise.reject(Object.assign(new Error(name), { name }))

    // pause() cutting a pending play() is not a failure
    audio(url("e.wav")).playResult = reject("AbortError")
    b.play("error")
    await flush()
    expect(warn).not.toHaveBeenCalled()

    audio(url("c.wav")).playResult = reject("NotAllowedError")
    b.play("click")
    clock.advance(200)
    b.play("click")
    await flush()
    expect(warn).toHaveBeenCalledTimes(1)
    expect(String(warn.mock.calls[0][0])).toContain("c.wav")
    expect(error).not.toHaveBeenCalled()
  })

  it("a player whose stream was cut is re-created once after a short delay", () => {
    const { b, clock, audio, warn, error } = bank({ click: url("c.wav") })
    const first = audio(url("c.wav"))
    b.play("click")
    first.fire("error")
    expect(warn).toHaveBeenCalledWith("Theme sound failed to load: c.wav")
    expect(error).not.toHaveBeenCalled()
    // the failed player's cut timer went with it; the reload waits
    expect(clock.pending()).toEqual([SOUND_RELOAD_MS])
    expect(b.play("click")).toBe(false)

    clock.advance(SOUND_RELOAD_MS)
    expect(FakeAudio.all).toHaveLength(2)
    const second = audio(url("c.wav"))
    expect(second).not.toBe(first)
    expect(first.src).toBe("")
    expect(second.preload).toBe("auto")
    expect(second.volume).toBe(SOUND_VOLUME)
    clock.advance(200)
    expect(b.play("click")).toBe(true)
    expect(second.plays).toBe(1)

    // it fails again before a good load: warn, no second reload
    second.fire("error")
    expect(warn).toHaveBeenCalledTimes(2)
    expect(clock.pending()).toEqual([])
    expect(FakeAudio.all).toHaveLength(2)
  })

  it("a good load re-arms the reload, up to a cap per URL for the session", () => {
    const session = createSoundSession()
    const { b, clock } = bank({ click: url("c.wav") }, "run", session)
    const latest = () => FakeAudio.all.at(-1)!
    for (let i = 0; i < MAX_SOUND_RELOADS; i++) {
      latest().fire("error")
      clock.advance(SOUND_RELOAD_MS)
      latest().fire("loadeddata")
    }
    expect(FakeAudio.all).toHaveLength(1 + MAX_SOUND_RELOADS)
    latest().fire("error")
    expect(clock.pending()).toEqual([])
    b.dispose()

    // a new bank (a theme change, the switch) shares the session's budget
    const again = bank({ click: url("c.wav") }, "run", session)
    FakeAudio.all.at(-1)!.fire("error")
    expect(again.clock.pending()).toEqual([])
    // another URL has its own
    const other = bank({ click: url("other.wav") }, "run", session)
    FakeAudio.all.at(-1)!.fire("error")
    expect(other.clock.pending()).toEqual([SOUND_RELOAD_MS])
  })

  it("a reload waits for run and never runs a timer under pause or off", () => {
    const { b, clock, audio } = bank({ click: url("c.wav") }, "pause")
    audio(url("c.wav")).fire("error")
    expect(clock.pending()).toEqual([])
    b.setMotion("run")
    expect(clock.pending()).toEqual([SOUND_RELOAD_MS])
    // leaving run again drops the timer and keeps the reload owed
    b.setMotion("off")
    expect(clock.pending()).toEqual([])
    clock.advance(SOUND_RELOAD_MS)
    expect(FakeAudio.all).toHaveLength(1)
    b.setMotion("run")
    clock.advance(SOUND_RELOAD_MS)
    expect(FakeAudio.all).toHaveLength(2)
  })

  it("dispose releases every player and clears every timer", () => {
    const { b, clock, audio } = bank({
      click: url("c.wav"),
      error: url("e.wav")
    })
    b.play("click")
    audio(url("e.wav")).fire("error")
    const players = [...FakeAudio.all]
    b.dispose()
    for (const a of players) {
      expect(a.paused).toBe(true)
      expect(a.src).toBe("")
      expect(a.loads).toBe(1)
    }
    expect(clock.pending()).toEqual([])
    expect(b.play("click")).toBe(false)
    // a late event from a released player does nothing
    players[0].fire("error")
    expect(clock.pending()).toEqual([])
  })

  it("a theme has sounds when any URL is set; equal URLs are the same sounds", () => {
    expect(hasSounds(noSounds())).toBe(false)
    expect(hasSounds(null)).toBe(false)
    for (const event of SOUND_EVENTS) {
      expect(hasSounds({ ...noSounds(), [event]: url("x.wav") })).toBe(true)
    }
    const a = { ...noSounds(), click: url("c.wav") }
    expect(sameSounds(a, { ...a })).toBe(true)
    expect(sameSounds(a, { ...a, click: url("d.wav") })).toBe(false)
    expect(sameSounds(null, null)).toBe(true)
    expect(sameSounds(a, null)).toBe(false)
  })
})

describe("task sounds", () => {
  it("a new launch task starts the launch sound", () => {
    const launch = task(7, "InstanceTaskLaunch")
    expect(launchesStarted([], [launch])).toBe(true)
    expect(launchesStarted([launch], [launch])).toBe(false)
    expect(launchesStarted(undefined, [launch])).toBe(false)
    expect(launchesStarted([], [task(8, "InstanceTaskPrepare")])).toBe(false)
  })

  it("a finished download plays once", () => {
    const prepare = task(3, "InstanceTaskPrepare")
    expect(downloadsFinished([prepare], [])).toBe(true)
    expect(downloadsFinished([], [])).toBe(false)
    expect(
      downloadsFinished([task(3, "InstanceTaskPrepare", failed)], [])
    ).toBe(false)
    expect(downloadsFinished([task(4, "InstanceTaskLaunch")], [])).toBe(false)
    expect(downloadsFinished(undefined, [])).toBe(false)
    // still running
    expect(downloadsFinished([prepare], [prepare])).toBe(false)
    expect(DOWNLOAD_TASKS).toEqual([
      "InstanceTaskPrepare",
      "InstanceTaskInstallMod",
      "ServerTaskInstall",
      "ServerTaskInstallFromModpack",
      "ServerTaskInstallMod"
    ])
    for (const name of DOWNLOAD_TASKS) {
      expect(downloadsFinished([task(9, name)], [])).toBe(true)
    }
  })
})

describe("toast sounds", () => {
  it("toasts map to notification or error", () => {
    expect(toastSound("error")).toBe("error")
    // a plain toast() has no data-type at all
    for (const type of ["success", "info", "warning", "default", null]) {
      expect(toastSound(type)).toBe("notification")
    }
    expect(toastSound("loading")).toBeNull()
  })

  it("a dismissed toast is not a new one", async () => {
    // somoto keys its list by object, and a dismiss swaps the toast for a
    // copy marked removed: a fresh node, which leaves after its exit.
    const region = document.createElement("div")
    const list = document.createElement("ol")
    region.appendChild(list)
    document.body.appendChild(region)
    const onToast = vi.fn()
    watchToasts(region, onToast)
    const flush = async () => {
      for (let i = 0; i < 3; i++) await Promise.resolve()
    }
    const toastEl = (type: string | null, removed = "false") => {
      const li = document.createElement("li")
      li.setAttribute("data-somoto-toast", "")
      if (type !== null) li.setAttribute("data-type", type)
      li.setAttribute("data-removed", removed)
      return li
    }

    const shown = toastEl("error")
    list.appendChild(shown)
    await flush()
    expect(onToast.mock.calls).toEqual([["error"]])

    // the dismiss: the old node goes, a copy marked removed comes in
    shown.remove()
    list.appendChild(toastEl("error", "true"))
    await flush()
    expect(onToast).toHaveBeenCalledTimes(1)

    // marked removed reactively, right after it went in
    const marked = toastEl("info")
    list.appendChild(marked)
    marked.setAttribute("data-removed", "true")
    await flush()
    expect(onToast).toHaveBeenCalledTimes(1)

    // in and out within the same task
    const blip = toastEl("success")
    list.appendChild(blip)
    blip.remove()
    await flush()
    expect(onToast).toHaveBeenCalledTimes(1)

    // a new toast still reports, and so does an update by id (a new node)
    const loading = toastEl("loading")
    list.appendChild(loading)
    await flush()
    expect(onToast).toHaveBeenLastCalledWith("loading")
    loading.replaceWith(toastEl("success"))
    await flush()
    expect(onToast).toHaveBeenLastCalledWith("success")
    list.appendChild(toastEl(null))
    await flush()
    expect(onToast.mock.calls).toEqual([
      ["error"],
      ["loading"],
      ["success"],
      [null]
    ])
  })

  it("new toasts are reported with their type", async () => {
    const region = document.createElement("div")
    document.body.appendChild(region)
    const onToast = vi.fn()
    const stop = watchToasts(region, onToast)
    const flush = async () => {
      // the observer's own microtask, then the read
      for (let i = 0; i < 3; i++) await Promise.resolve()
    }

    const li = document.createElement("li")
    li.setAttribute("data-somoto-toast", "")
    region.appendChild(li)
    // somoto sets the type reactively, after the node is in place
    li.setAttribute("data-type", "error")
    await flush()
    expect(onToast).toHaveBeenCalledTimes(1)
    expect(onToast).toHaveBeenCalledWith("error")

    // moving the same toast is not a new one
    region.appendChild(li)
    await flush()
    expect(onToast).toHaveBeenCalledTimes(1)

    const wrapper = document.createElement("ol")
    wrapper.innerHTML = '<li data-somoto-toast data-type="success"></li>'
    region.appendChild(wrapper)
    await flush()
    expect(onToast).toHaveBeenCalledTimes(2)
    expect(onToast).toHaveBeenLastCalledWith("success")

    // something that isn't a toast says nothing
    region.appendChild(document.createElement("span"))
    await flush()
    expect(onToast).toHaveBeenCalledTimes(2)

    stop()
    const late = document.createElement("li")
    late.setAttribute("data-somoto-toast", "")
    late.setAttribute("data-type", "info")
    region.appendChild(late)
    await flush()
    expect(onToast).toHaveBeenCalledTimes(2)
  })

  it("a toast added just before the disconnect says nothing", async () => {
    const region = document.createElement("div")
    document.body.appendChild(region)
    const onToast = vi.fn()
    const stop = watchToasts(region, onToast)
    const li = document.createElement("li")
    li.setAttribute("data-somoto-toast", "")
    region.appendChild(li)
    // the observer has queued its record but the read hasn't run yet
    await Promise.resolve()
    stop()
    for (let i = 0; i < 3; i++) await Promise.resolve()
    expect(onToast).not.toHaveBeenCalled()
  })
})

describe("click sounds", () => {
  it("only enabled primary buttons click", () => {
    document.body.innerHTML = `
      <button id="p" data-gdl="button button.primary"><span id="inner">Go</span></button>
      <button id="d" data-gdl="button button.primary button.disabled">Off</button>
      <button id="s" data-gdl="button button.secondary">No</button>
    `
    const byId = (id: string) => document.getElementById(id)
    expect(isPrimaryButton(byId("inner"))).toBe(true)
    expect(isPrimaryButton(byId("p"))).toBe(true)
    expect(isPrimaryButton(byId("inner")!.firstChild)).toBe(true)
    expect(isPrimaryButton(byId("d"))).toBe(false)
    expect(isPrimaryButton(byId("s"))).toBe(false)
    expect(isPrimaryButton(null)).toBe(false)
    expect(isPrimaryButton(document)).toBe(false)
  })
})
