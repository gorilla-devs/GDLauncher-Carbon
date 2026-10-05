import { expect, it, vi } from "vitest"
import fs from "node:fs"
import path from "node:path"
import { EventEmitter } from "node:events"
import {
  SHUTDOWN_PROMPT_ENGLISH,
  ShutdownPrompts,
  nativeShutdownDialog,
  shutdownPromptLines,
  shutdownPromptText
} from "../shutdownPrompt"
import { subscribeShutdownPrompt } from "../../../preload/shutdownPromptBridge"
import type { ShutdownView } from "../shutdownFlow"

const view = (overrides: Partial<ShutdownView> = {}): ShutdownView => ({
  action: "quit",
  stage: "blocked",
  reason: "unresolved",
  blockers: [
    { kind: "deletion", name: "delete instance 12" },
    { kind: "server", name: "server 7 run" },
    { kind: "server", name: "server 8 run" }
  ],
  canWait: true,
  finishing: false,
  serversStopped: true,
  ...overrides
})
function window() {
  const sent: unknown[][] = []
  return {
    sent,
    shown: 0,
    focused: 0,
    destroyed: false,
    isDestroyed() {
      return this.destroyed
    },
    show() {
      this.shown++
    },
    focus() {
      this.focused++
    },
    webContents: {
      send: (...args: unknown[]) => {
        sent.push(args)
      }
    }
  }
}

it("a mounted App shows the prompt in the window and its answer resolves it", async () => {
  const native = vi.fn()
  const prompts = new ShutdownPrompts(native)
  const w = window()
  prompts.receiver(w.webContents, true)
  const asked = prompts.ask(w, view(), new AbortController().signal)
  expect(prompts.open).toBe(true)
  expect(w.shown).toBe(1)
  expect(w.focused).toBe(1)
  expect(w.sent).toEqual([["shutdown-prompt", { id: 1, view: view() }]])
  prompts.shown(w.webContents, 1)
  // Another renderer, a wrong id or an unknown choice changes nothing.
  prompts.answer({}, 1, "keep")
  prompts.answer(w.webContents, 2, "keep")
  prompts.answer(w.webContents, 1, "explode")
  expect(prompts.open).toBe(true)
  prompts.answer(w.webContents, 1, "force")
  expect(await asked).toBe("force")
  expect(prompts.open).toBe(false)
  expect(native).not.toHaveBeenCalled()
})

it("a prompt closes itself when aborted and when its App goes away", async () => {
  const prompts = new ShutdownPrompts(vi.fn())
  const w = window()
  prompts.receiver(w.webContents, true)
  const controller = new AbortController()
  const asked = prompts.ask(w, view(), controller.signal)
  controller.abort()
  expect(await asked).toBeNull()
  expect(w.sent.at(-1)).toEqual(["shutdown-prompt", { id: 1, view: null }])

  const again = prompts.ask(w, view(), new AbortController().signal)
  prompts.receiver(w.webContents, false)
  expect(await again).toBeNull()
  expect(prompts.open).toBe(false)
})

it("a prompt the App does not report on screen is asked with the native dialog instead", async () => {
  vi.useFakeTimers()
  try {
    let choose!: (choice: "force") => void
    const native = vi.fn(
      () => new Promise<"force">((resolve) => (choose = resolve))
    )
    const prompts = new ShutdownPrompts(native, 2000)
    const w = window()
    prompts.receiver(w.webContents, true)
    const signal = new AbortController().signal
    const asked = prompts.ask(w, view(), signal)
    // Reports for another renderer or another prompt do not count.
    prompts.shown({}, 1)
    prompts.shown(w.webContents, 2)
    await vi.advanceTimersByTimeAsync(1999)
    expect(native).not.toHaveBeenCalled()
    // The window's close button stays held: a prompt is still open.
    expect(prompts.open).toBe(true)
    await vi.advanceTimersByTimeAsync(1)
    expect(native).toHaveBeenCalledWith(w, view(), signal)
    expect(prompts.open).toBe(true)
    // The App's prompt is closed, in case it shows up late.
    expect(w.sent.at(-1)).toEqual(["shutdown-prompt", { id: 1, view: null }])
    // A late answer from the App no longer counts.
    prompts.answer(w.webContents, 1, "keep")
    choose("force")
    expect(await asked).toBe("force")
    expect(prompts.open).toBe(false)

    // Reported on screen in time: no native dialog.
    native.mockClear()
    const shown = prompts.ask(w, view(), new AbortController().signal)
    prompts.shown(w.webContents, 2)
    await vi.advanceTimersByTimeAsync(10000)
    expect(native).not.toHaveBeenCalled()
    prompts.answer(w.webContents, 2, "wait")
    expect(await shown).toBe("wait")
  } finally {
    vi.useRealTimers()
  }
})

it("a prompt on screen follows what the stop waits for under the same id, and a native dialog asks what it says last", async () => {
  vi.useFakeTimers()
  try {
    const native = vi.fn(async () => "wait" as const)
    const prompts = new ShutdownPrompts(native, 2000)
    const w = window()
    prompts.receiver(w.webContents, true)
    const signal = new AbortController().signal
    const asked = prompts.ask(w, view({ stage: "waiting" }), signal)
    const later = view({
      stage: "waiting",
      blockers: [{ kind: "deletion", name: "delete instance 12" }]
    })
    prompts.update(signal, later)
    // Another prompt's update changes nothing here.
    prompts.update(new AbortController().signal, view({ blockers: [] }))
    expect(w.sent).toEqual([
      ["shutdown-prompt", { id: 1, view: view({ stage: "waiting" }) }],
      ["shutdown-prompt", { id: 1, view: later }]
    ])
    await vi.advanceTimersByTimeAsync(2000)
    expect(native).toHaveBeenCalledWith(w, later, signal)
    expect(await asked).toBe("wait")
    // Nothing is sent for a prompt that was answered.
    prompts.update(signal, view())
    expect(w.sent).toHaveLength(3)
  } finally {
    vi.useRealTimers()
  }
})

it("a stopping prompt has nothing to choose, holds the close button and is never a native dialog", async () => {
  vi.useFakeTimers()
  try {
    const stopping = view({ stage: "stopping", reason: null, blockers: [] })
    const lines = shutdownPromptLines({ ...stopping, action: "restart" })
    expect(lines).toEqual({
      title: "title_restart_waiting",
      body: "body_stopping",
      rows: [],
      notes: [],
      buttons: []
    })
    const native = vi.fn()
    const prompts = new ShutdownPrompts(native, 2000)
    const w = window()

    // No App (the loading screen): nothing is shown, the close button is
    // held until the core has exited.
    const quiet = new AbortController()
    const held = prompts.ask(w, stopping, quiet.signal)
    expect(prompts.open).toBe(true)
    quiet.abort()
    expect(await held).toBeNull()
    expect(prompts.open).toBe(false)

    // An App that does not report it on screen: it just goes away.
    prompts.receiver(w.webContents, true)
    const unseen = prompts.ask(w, stopping, new AbortController().signal)
    await vi.advanceTimersByTimeAsync(2000)
    expect(await unseen).toBeNull()
    expect(prompts.open).toBe(false)
    expect(w.sent.at(-1)).toEqual(["shutdown-prompt", { id: 1, view: null }])
    expect(native).not.toHaveBeenCalled()
  } finally {
    vi.useRealTimers()
  }
})

it("without a mounted App the native dialog asks, parented to the launcher window", async () => {
  const native = vi.fn(async () => "keep" as const)
  const prompts = new ShutdownPrompts(native)
  const w = window()
  const signal = new AbortController().signal
  expect(await prompts.ask(w, view(), signal)).toBe("keep")
  expect(native).toHaveBeenCalledWith(w, view(), signal)
  expect(w.shown).toBe(1)
  expect(w.sent).toEqual([])
})

it("the native dialog lists each kind once, says what Quit now does and maps buttons to choices", () => {
  const { options, choices } = nativeShutdownDialog(view())
  expect(options.title).toBe("GDLauncher can't close yet")
  expect(options.buttons).toEqual([
    "Try again",
    "Quit now",
    "Keep GDLauncher open"
  ])
  expect(choices).toEqual(["wait", "force", "keep"])
  expect(options.cancelId).toBe(2)
  expect(options.detail.match(/still stopping and saving/g)).toHaveLength(1)
  expect(options.detail).toContain(SHUTDOWN_PROMPT_ENGLISH.deletion_warning)
  expect(options.detail).toContain(
    "Servers that are still stopping will finish in the background."
  )
  expect(options.detail).toContain(SHUTDOWN_PROMPT_ENGLISH.servers_stopped)

  const update = nativeShutdownDialog(
    view({
      action: "update",
      blockers: [{ kind: "unconfirmed", name: "lost worker" }],
      canWait: false,
      serversStopped: false
    })
  )
  expect(update.options.buttons).toEqual([
    "Install now",
    "Keep GDLauncher open"
  ])
  expect(update.choices).toEqual(["force", "keep"])
  expect(update.options.detail).not.toContain(
    SHUTDOWN_PROMPT_ENGLISH.servers_stopped
  )
})

it("Keep GDLauncher open is offered only while the core can still go back, and Escape never forces", () => {
  // The core agreed to stop; only its exit was not seen yet.
  const exit = nativeShutdownDialog(view({ reason: "exit", blockers: [] }))
  expect(exit.options.buttons).toEqual(["Try again", "Quit now"])
  expect(exit.choices[exit.options.cancelId]).toBe("wait")
  expect(exit.options.detail).not.toContain(
    SHUTDOWN_PROMPT_ENGLISH.servers_stopped
  )

  // Keep GDLauncher open came too late: the update carries on.
  const late = shutdownPromptLines(
    view({
      action: "update",
      stage: "waiting",
      reason: null,
      canWait: false,
      finishing: true
    })
  )
  expect(late.buttons.map((button) => button.choice)).toEqual(["force"])
  expect(late.notes[0]).toBe("too_late_to_keep")
  expect(late.notes).not.toContain("servers_stopped")

  // Waiting on a quit: Escape waits; Keep GDLauncher open stays the cancel.
  const waiting = nativeShutdownDialog(view({ stage: "waiting", reason: null }))
  expect(waiting.choices).toEqual(["wait", "force", "keep"])
  expect(waiting.options.cancelId).toBe(2)
  expect(waiting.options.detail).toContain(
    SHUTDOWN_PROMPT_ENGLISH.wait_hint_quit
  )
  // An update's waiting prompt is the wait itself.
  const updating = nativeShutdownDialog(
    view({ action: "update", stage: "waiting", reason: null, canWait: false })
  )
  expect(updating.choices).toEqual(["force", "keep"])
  expect(updating.options.detail).not.toContain(
    SHUTDOWN_PROMPT_ENGLISH.wait_hint_quit
  )
})

it("Keep GDLauncher open says the installs, launches and tasks it lists were already stopped, unless it is too late to keep", () => {
  for (const kind of ["install", "game", "task"] as const) {
    const lines = shutdownPromptLines(
      view({
        stage: "waiting",
        reason: null,
        blockers: [{ kind, name: "work " + kind }],
        serversStopped: false
      })
    )
    expect(lines.buttons.map((button) => button.choice)).toContain("keep")
    expect(lines.notes, kind).toContain("work_cancelled")
  }
  // The native dialog says it too, and next to the servers' note.
  const dialog = nativeShutdownDialog(
    view({ blockers: [{ kind: "install", name: "modpack 3" }] })
  )
  expect(dialog.options.detail).toContain(
    SHUTDOWN_PROMPT_ENGLISH.work_cancelled
  )
  expect(dialog.options.detail).toContain(
    SHUTDOWN_PROMPT_ENGLISH.servers_stopped
  )
  // Nothing of that kind is listed: a deletion and servers are not cancelled.
  expect(shutdownPromptLines(view()).notes).not.toContain("work_cancelled")
  // A quit that is finishing offers no Keep, so the note has nothing to say.
  const finishing = shutdownPromptLines(
    view({
      stage: "waiting",
      reason: null,
      blockers: [{ kind: "install", name: "modpack 3" }],
      canWait: false,
      finishing: true
    })
  )
  expect(finishing.buttons.map((button) => button.choice)).not.toContain("keep")
  expect(finishing.notes).not.toContain("work_cancelled")
  const exit = shutdownPromptLines(
    view({ reason: "exit", blockers: [{ kind: "task", name: "export" }] })
  )
  expect(exit.notes).not.toContain("work_cancelled")
})

it("a running Pre-Launch or Post-Exit Hook is named as one, is not called stopped, and Quit now warns it can cut the script short", () => {
  const waiting = shutdownPromptLines(
    view({
      stage: "waiting",
      reason: null,
      blockers: [{ kind: "hook", name: "temporary child" }],
      serversStopped: false
    })
  )
  expect(waiting.rows).toEqual(["blocker_hook"])
  expect(waiting.notes).toContain("hook_warning")
  // The quit leaves the script running, so it was not asked to stop.
  expect(waiting.notes).not.toContain("work_cancelled")
  expect(waiting.buttons.map((button) => button.choice)).toEqual([
    "wait",
    "force",
    "keep"
  ])
  const dialog = nativeShutdownDialog(
    view({
      blockers: [
        { kind: "hook", name: "temporary child" },
        { kind: "install", name: "modpack 3" }
      ]
    })
  )
  expect(dialog.options.detail).toContain(SHUTDOWN_PROMPT_ENGLISH.blocker_hook)
  expect(dialog.options.detail).toContain(SHUTDOWN_PROMPT_ENGLISH.hook_warning)
  expect(dialog.options.detail).toContain(
    SHUTDOWN_PROMPT_ENGLISH.blocker_install
  )
  expect(dialog.options.detail).not.toContain(
    SHUTDOWN_PROMPT_ENGLISH.blocker_task
  )
  // No hook, no warning about one.
  expect(shutdownPromptLines(view()).notes).not.toContain("hook_warning")
})

it("the native dialog uses the text the App sent, and only known keys of it", () => {
  const text = shutdownPromptText({
    force_quit: "Esci ora",
    keep_open: "Tieni aperto GDLauncher",
    title_quit_blocked: "GDLauncher non può ancora chiudersi",
    injected: "<b>nope</b>",
    wait: 3
  })
  expect(Object.keys(text).sort()).toEqual([
    "force_quit",
    "keep_open",
    "title_quit_blocked"
  ])
  const { options } = nativeShutdownDialog(view({ canWait: false }), text)
  expect(options.title).toBe("GDLauncher non può ancora chiudersi")
  expect(options.buttons).toEqual(["Esci ora", "Tieni aperto GDLauncher"])
})

it("every prompt line is a key the locale files translate, with the same English", () => {
  const english = JSON.parse(
    fs.readFileSync(
      path.resolve(
        __dirname,
        "../../../../../../packages/i18n/locale/english/app.json"
      ),
      "utf8"
    )
  )
  for (const [key, text] of Object.entries(SHUTDOWN_PROMPT_ENGLISH))
    expect(english[`_trn_shutdown_prompt.${key}`], key).toBe(text)
  for (const action of [
    "quit",
    "update",
    "restart",
    "reset",
    "restore"
  ] as const)
    for (const [stage, finishing] of [
      ["waiting", false],
      ["blocked", false],
      ["waiting", true],
      ["stopping", false]
    ] as const) {
      const lines = shutdownPromptLines(view({ action, stage, finishing }))
      for (const key of [
        lines.title,
        lines.body,
        ...lines.rows,
        ...lines.notes,
        ...lines.buttons.map((button) => button.key)
      ])
        expect(SHUTDOWN_PROMPT_ENGLISH, key).toHaveProperty(key)
    }
})

it("the preload bridge acknowledges once per document and withdraws after the last listener", () => {
  const ipc = Object.assign(new EventEmitter(), { send: vi.fn() })
  const received: unknown[] = []
  const first = subscribeShutdownPrompt(ipc as any, (m) => received.push(m))
  const second = subscribeShutdownPrompt(ipc as any, () => {})
  expect(ipc.send.mock.calls).toEqual([["shutdown-prompt-receiver", true]])
  ipc.emit("shutdown-prompt", { sender: "event" }, { id: 1, view: null })
  expect(received).toEqual([{ id: 1, view: null }])
  first()
  first()
  expect(ipc.send).toHaveBeenCalledTimes(1)
  second()
  expect(ipc.send.mock.calls.at(-1)).toEqual([
    "shutdown-prompt-receiver",
    false
  ])
})
