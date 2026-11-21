// @vitest-environment jsdom
import { describe, it, expect, vi, afterEach } from "vitest"
import { createRoot, createSignal, onCleanup } from "solid-js"
import type {
  FETask,
  FEThemeSounds,
  Translation
} from "@gd/core_module/bindings"
import { wireSounds, type SoundsDocument, type TaskList } from "../soundsWiring"
import type { SoundBank, SoundEvent } from "../sounds"
import type { MotionState } from "../motion"

afterEach(() => {
  vi.restoreAllMocks()
  document.body.innerHTML = ""
})

const sounds = (patch: Partial<FEThemeSounds> = {}): FEThemeSounds => ({
  launch: null,
  downloadComplete: null,
  click: null,
  notification: null,
  error: null,
  ...patch
})

const task = (id: number, translation: string): FETask => ({
  id,
  name: { translation, args: { name: "pack" } } as unknown as Translation,
  progress: { type: "Known", value: 0.5 },
  downloaded: 0,
  download_total: 0,
  active_subtasks: []
})

interface FakeBank extends SoundBank {
  plays: SoundEvent[]
  motions: MotionState[]
  disposed: boolean
}

function setup(opts: {
  prefs?: { sounds: boolean; effects?: boolean; tileSizeChosen?: boolean }
  theme?: FEThemeSounds | null
  region?: boolean
}) {
  vi.spyOn(console, "warn").mockImplementation(() => {})
  if (opts.region !== false) {
    document.body.innerHTML = '<div data-gdl="toast-region"></div>'
  }
  const [prefs, setPrefs] = createSignal<
    { sounds: boolean; effects?: boolean; tileSizeChosen?: boolean } | undefined
  >(opts.prefs)
  const [theme, setTheme] = createSignal<FEThemeSounds | null>(
    opts.theme ?? null
  )
  const [motion, setMotion] = createSignal<MotionState>("run")
  const [tasks, setTasks] = createSignal<TaskList>({
    data: undefined,
    dataUpdatedAt: 0
  })
  let wall = 10_000
  const banks: FakeBank[] = []
  const taskLists = { made: 0, dropped: 0 }
  const listeners = { added: 0, removed: 0 }
  // the real document, counting the click listeners
  const doc: SoundsDocument = {
    addEventListener(type, listener, capture) {
      listeners.added++
      document.addEventListener(type, listener, capture)
    },
    removeEventListener(type, listener, capture) {
      listeners.removed++
      document.removeEventListener(type, listener, capture)
    },
    querySelector: (selector) => document.querySelector(selector)
  }

  const dispose = createRoot((d) => {
    wireSounds({
      preferences: prefs,
      sounds: theme,
      motion,
      createBank: (_s, state) => {
        const bank: FakeBank = {
          plays: [],
          motions: [state],
          disposed: false,
          play(event) {
            bank.plays.push(event)
            return true
          },
          setMotion(next) {
            bank.motions.push(next)
          },
          dispose() {
            bank.disposed = true
          }
        }
        banks.push(bank)
        return bank
      },
      createTaskList: () => {
        taskLists.made++
        onCleanup(() => taskLists.dropped++)
        return {
          get data() {
            return tasks().data
          },
          get dataUpdatedAt() {
            return tasks().dataUpdatedAt
          }
        }
      },
      now: () => wall,
      document: doc
    })
    return d
  })
  const live = () => banks.filter((b) => !b.disposed)
  return {
    dispose,
    banks,
    live,
    taskLists,
    listeners,
    setPrefs,
    setTheme,
    setMotion,
    setTasks,
    advanceWall: (ms: number) => (wall += ms)
  }
}

const all = sounds({
  launch: "l.wav",
  downloadComplete: "d.wav",
  click: "c.wav",
  notification: "n.wav",
  error: "e.wav"
})

describe("sound wiring", () => {
  it("nothing is made while sounds are off or the theme has none", () => {
    const off = setup({ prefs: { sounds: false }, theme: all })
    expect(off.banks).toHaveLength(0)
    expect(off.taskLists.made).toBe(0)
    expect(off.listeners.added).toBe(0)
    off.dispose()

    const silent = setup({ prefs: { sounds: true }, theme: sounds() })
    expect(silent.banks).toHaveLength(0)
    expect(silent.taskLists.made).toBe(0)
    silent.dispose()

    const loading = setup({ prefs: undefined, theme: all })
    expect(loading.banks).toHaveLength(0)
    loading.dispose()
  })

  it("only the inputs the theme's sounds need are added", () => {
    const t = setup({
      prefs: { sounds: true },
      theme: sounds({ launch: "l.wav" })
    })
    expect(t.banks).toHaveLength(1)
    expect(t.taskLists.made).toBe(1)
    expect(t.listeners.added).toBe(0)
    t.dispose()
    const c = setup({ prefs: { sounds: true }, theme: sounds({ click: "c" }) })
    expect(c.taskLists.made).toBe(0)
    expect(c.listeners.added).toBe(1)
    c.dispose()
  })

  it("writing another preference keeps the players", () => {
    const t = setup({
      prefs: { sounds: true, effects: true, tileSizeChosen: false },
      theme: all
    })
    expect(t.banks).toHaveLength(1)
    // CustomizeTheme's setQueryData, a refetch, Task 5's tile size pick
    t.setPrefs({ sounds: true, effects: false, tileSizeChosen: false })
    t.setPrefs({ sounds: true, effects: false, tileSizeChosen: true })
    t.setPrefs({ sounds: true, effects: false, tileSizeChosen: true })
    expect(t.banks).toHaveLength(1)
    expect(t.live()).toHaveLength(1)
    expect(t.taskLists).toEqual({ made: 1, dropped: 0 })
    expect(t.listeners).toEqual({ added: 1, removed: 0 })

    t.setPrefs({ sounds: false, effects: false, tileSizeChosen: true })
    expect(t.live()).toHaveLength(0)
    expect(t.taskLists).toEqual({ made: 1, dropped: 1 })
    expect(t.listeners).toEqual({ added: 1, removed: 1 })
    t.dispose()
  })

  it("an equal theme keeps the players; other files rebuild them", () => {
    const t = setup({ prefs: { sounds: true }, theme: all })
    t.setTheme({ ...all })
    expect(t.banks).toHaveLength(1)
    t.setTheme({ ...all, click: "c2.wav" })
    expect(t.banks).toHaveLength(2)
    expect(t.banks[0].disposed).toBe(true)
    expect(t.live()).toHaveLength(1)
    t.setTheme(sounds())
    expect(t.live()).toHaveLength(0)
    t.dispose()
  })

  it("the bank follows the motion state", () => {
    const t = setup({ prefs: { sounds: true }, theme: all })
    t.setMotion("pause")
    t.setMotion("run")
    expect(t.banks[0].motions.at(-2)).toBe("pause")
    expect(t.banks[0].motions.at(-1)).toBe("run")
    t.dispose()
    expect(t.banks[0].disposed).toBe(true)
  })

  it("primary clicks and new toasts play", async () => {
    const t = setup({ prefs: { sounds: true }, theme: all })
    document.body.insertAdjacentHTML(
      "beforeend",
      '<button data-gdl="button button.primary">Go</button>'
    )
    document.querySelector("button")!.click()
    const li = document.createElement("li")
    li.setAttribute("data-somoto-toast", "")
    li.setAttribute("data-type", "error")
    document.querySelector('[data-gdl="toast-region"]')!.appendChild(li)
    for (let i = 0; i < 3; i++) await Promise.resolve()
    expect(t.banks[0].plays).toEqual(["click", "error"])
    t.dispose()
  })

  it("a missing toast region warns once and leaves toasts silent", () => {
    const t = setup({ prefs: { sounds: true }, theme: all, region: false })
    t.setTheme({ ...all, error: "e2.wav" })
    expect(t.banks).toHaveLength(2)
    expect(console.warn).toHaveBeenCalledTimes(1)
    t.dispose()
  })

  it("the task baseline is the first list fetched after sounds turn on", () => {
    const t = setup({ prefs: { sounds: true }, theme: all })
    const bank = t.banks[0]
    const prepare = task(3, "InstanceTaskPrepare")
    // the cache still holds a list from before: the download in it may have
    // finished meanwhile, and nothing must play for it
    t.setTasks({ data: [prepare], dataUpdatedAt: 9_000 })
    t.advanceWall(50)
    t.setTasks({ data: [], dataUpdatedAt: 10_050 })
    expect(bank.plays).toEqual([])

    // from there on, changes play
    t.setTasks({ data: [task(7, "InstanceTaskLaunch")], dataUpdatedAt: 10_100 })
    t.setTasks({
      data: [task(7, "InstanceTaskLaunch"), task(8, "InstanceTaskPrepare")],
      dataUpdatedAt: 10_200
    })
    t.setTasks({ data: [task(7, "InstanceTaskLaunch")], dataUpdatedAt: 10_300 })
    expect(bank.plays).toEqual(["launch", "downloadComplete"])

    // a new watcher starts over: its own first fetched list plays nothing.
    // While sounds were off a download started, and the cache still holds
    // that list when they come back on; it finished before the next fetch.
    t.setPrefs({ sounds: false })
    t.setTasks({
      data: [task(7, "InstanceTaskLaunch"), task(9, "InstanceTaskPrepare")],
      dataUpdatedAt: 10_900
    })
    t.advanceWall(1_000)
    t.setPrefs({ sounds: true })
    const again = t.banks[1]
    t.setTasks({ data: [task(7, "InstanceTaskLaunch")], dataUpdatedAt: 11_400 })
    expect(again.plays).toEqual([])
    // and the watcher is live: the next download that ends plays
    t.setTasks({
      data: [task(7, "InstanceTaskLaunch"), task(10, "InstanceTaskPrepare")],
      dataUpdatedAt: 11_500
    })
    t.setTasks({ data: [task(7, "InstanceTaskLaunch")], dataUpdatedAt: 11_600 })
    expect(again.plays).toEqual(["downloadComplete"])
    t.dispose()
  })
})
