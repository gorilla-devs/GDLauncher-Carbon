// @vitest-environment jsdom
import { it, expect, vi, beforeEach } from "vitest"
import path from "node:path"
import { RESET_CONFIRM_GUARD_MS } from "./resetButton"
const bridge = vi.hoisted(() => ({
  api: {} as Record<string, any>,
  invoke: vi.fn(),
  on: vi.fn(),
  send: vi.fn()
}))
vi.mock("electron", () => ({
  contextBridge: {
    exposeInMainWorld: (name: string, value: any) => {
      bridge.api[name] = value
    }
  },
  ipcRenderer: { invoke: bridge.invoke, on: bridge.on, send: bridge.send }
}))
vi.mock("./utils", () => ({ domReady: async () => true }))
let clock = 0
/** Clicks the reset button, waits past the double-click guard and confirms. */
function confirmReset() {
  document.querySelector<HTMLButtonElement>("#reset-db-btn")!.click()
  clock += RESET_CONFIRM_GUARD_MS
  document.querySelector<HTMLButtonElement>("#reset-db-confirm-btn")!.click()
}
const resets = () =>
  bridge.invoke.mock.calls.filter((call) => call[0] === "deleteDbAndRestart")
    .length
async function ready() {
  clock = 0
  vi.spyOn(Date, "now").mockImplementation(() => clock)
  document.body.innerHTML = '<div id="appFatalCrash"></div>'
  vi.stubGlobal("__APP_VERSION__", "synthetic")
  bridge.invoke.mockReset()
  bridge.on.mockReset()
  bridge.send.mockReset()
  bridge.invoke.mockImplementation(async (name: string) =>
    name === "getRuntimePath"
      ? "/synthetic/runtime"
      : name === "getUpdateState"
        ? {
            state: "unsupported",
            revision: 1,
            error: { message: "controlled" }
          }
        : undefined
  )
  vi.resetModules()
  await import("./loading")
  await Promise.resolve()
}
beforeEach(ready)
it("fatal recovery removes universal reset advice and describes locked database cause", async () => {
  await bridge.api.fatalError(
    "synthetic failure",
    "CoreModule",
    undefined,
    "locked"
  )
  expect(document.body.textContent).not.toContain("Most common fix")
  expect(document.body.textContent).toContain("lock")
  await bridge.api.fatalError(
    "synthetic failure",
    "CoreModule",
    undefined,
    "corruption"
  )
  expect(document.body.textContent).toContain(
    "come back from their folders with their settings"
  )
  expect(document.body.textContent).toContain("recovery copy")
})
it.each(["fatal", "backwards"])(
  "%s reset button shows failure and allows retry",
  async (screen) => {
    await (screen === "fatal"
      ? bridge.api.fatalError("failure", "CoreModule")
      : bridge.api.backwardsMigrationError())
    bridge.invoke.mockImplementation(async () => ({
      ok: false,
      message: "Retained <foreign> bytes; retry",
      recoveryPath: "/synthetic/backup"
    }))
    confirmReset()
    await new Promise((resolve) => setTimeout(resolve, 0))
    const button = document.querySelector<HTMLButtonElement>("#reset-db-btn")!
    expect(button.disabled).toBe(false)
    expect(document.body.textContent).toContain(
      "Retained <foreign> bytes; retry"
    )
    expect(document.querySelector("foreign")).toBeNull()
  }
)

it.each([
  ["access", "permissions"],
  ["storage", "free space"],
  ["corruption", "corrupt"],
  ["version", "compatible"],
  ["downgrade", "downgrade"],
  ["unknown", "known cause"],
  [undefined, "last resort"]
])(
  "%s guidance is visible without likelihood advice",
  async (category, expected) => {
    await bridge.api.fatalError(
      "synthetic failure",
      "Window",
      undefined,
      category
    )
    expect(document.body.textContent).toContain(expected)
    expect(document.body.textContent).not.toContain(
      "This fixes most launch issues"
    )
  }
)
it("Restart, Open Folder and supplied Restore keep their actual IPC controls", async () => {
  await bridge.api.fatalError("failure", "CoreModule")
  expect(document.querySelector("#restore-snapshot-btn")).toBeNull()
  document.querySelector<HTMLButtonElement>("#restart-btn")!.click()
  expect(bridge.invoke).toHaveBeenCalledWith("relaunch")
  document.querySelector<HTMLButtonElement>("#open-db-folder-btn")!.click()
  expect(bridge.invoke).toHaveBeenCalledWith(
    "openFolder",
    `/synthetic/runtime${path.sep}gdl_conf.db`
  )
  await bridge.api.fatalError(
    "failure",
    "CoreModule",
    "/synthetic/runtime/gdl_conf.pre-downgrade.db",
    "downgrade"
  )
  expect(document.querySelector("#restore-snapshot-btn")).not.toBeNull()
})
it.each(["fatal"])(
  "%s rejected reset IPC is safe and retryable; success remains busy",
  async () => {
    await bridge.api.fatalError("failure", "CoreModule")
    bridge.invoke.mockRejectedValue(Error("controlled refusal"))
    const button = document.querySelector<HTMLButtonElement>("#reset-db-btn")!
    confirmReset()
    await new Promise((resolve) => setTimeout(resolve, 0))
    expect(button.disabled).toBe(false)
    expect(button.textContent).toBe("Retry Reset")
    bridge.invoke.mockReset()
    bridge.invoke.mockResolvedValue({
      ok: true,
      recoveryPath: "/synthetic/retained"
    })
    // A retry is confirmed again: one click on it resets nothing.
    button.click()
    expect(resets()).toBe(0)
    button.click()
    expect(button.textContent).toBe("Retry Reset")
    confirmReset()
    await new Promise((resolve) => setTimeout(resolve, 0))
    expect(resets()).toBe(1)
    expect(button.disabled).toBe(true)
  }
)

it("an interrupted restore offers Recover Database, which reports a refusal and allows a retry", async () => {
  await bridge.api.fatalError(
    "Database restore needs recovery",
    "CoreModule",
    undefined,
    "interrupted_restore"
  )
  expect(document.body.textContent).toContain("Recover Database")
  expect(document.querySelector("#db-recovery-advice")!.textContent).toContain(
    "did not finish"
  )
  bridge.invoke.mockImplementation(async () => ({
    ok: false,
    message: "Keep the files and ask for help"
  }))
  const button = document.querySelector<HTMLButtonElement>(
    "#recover-restore-btn"
  )!
  button.click()
  await new Promise((resolve) => setTimeout(resolve, 0))
  expect(bridge.invoke).toHaveBeenCalledWith("recoverInterruptedDbRestore")
  expect(button.disabled).toBe(false)
  expect(
    document.querySelector("#recover-restore-error")!.textContent
  ).toContain("Keep the files and ask for help")
})
it("Recover Database stays busy through the restart when it worked", async () => {
  await bridge.api.fatalError(
    "x",
    "CoreModule",
    undefined,
    "interrupted_restore"
  )
  bridge.invoke.mockImplementation(async () => ({ ok: true, restored: true }))
  const button = document.querySelector<HTMLButtonElement>(
    "#recover-restore-btn"
  )!
  button.click()
  await new Promise((resolve) => setTimeout(resolve, 0))
  expect(button.disabled).toBe(true)
})
it("no other startup failure offers Recover Database", async () => {
  await bridge.api.fatalError("x", "CoreModule", undefined, "corruption")
  expect(document.querySelector("#recover-restore-btn")).toBeNull()
})
it("the reset warning says how long the recovery copy is kept and that it needs room", async () => {
  await bridge.api.fatalError("x", "CoreModule", undefined, "corruption")
  expect(document.body.textContent).toContain("30 days")
  expect(document.body.textContent).toContain("room")
})
it("the fatal screen leaves the update channel to main, which never downgrades from it", async () => {
  await bridge.api.fatalError("synthetic failure", "CoreModule")
  document.querySelector<HTMLButtonElement>("#check-updates-btn")!.click()
  await new Promise((resolve) => setTimeout(resolve, 0))
  const calls = bridge.invoke.mock.calls.map((call) => call[0])
  expect(bridge.invoke).toHaveBeenCalledWith("checkForUpdatesFromRecovery")
  expect(calls).not.toContain("checkForUpdates")
  expect(calls).not.toContain("getSettings")
})

const screens = {
  fatal: () => bridge.api.fatalError("failure", "CoreModule"),
  window: () => bridge.api.fatalError("failure", "Window")
}
const resetButtons = () => ({
  button: document.querySelector<HTMLButtonElement>("#reset-db-btn")!,
  confirm: document.querySelector<HTMLButtonElement>("#reset-db-confirm-btn")!
})
it.each(Object.keys(screens) as (keyof typeof screens)[])(
  "one click on the %s screen's reset button resets nothing, and Cancel puts the button back",
  async (screen) => {
    await screens[screen]()
    const { button, confirm } = resetButtons()
    const label = button.textContent
    const background = button.style.background
    expect(confirm.style.display).toBe("none")
    button.click()
    expect(resets()).toBe(0)
    expect(confirm.style.display).toBe("")
    expect(confirm.textContent.trim()).toBe("Reset Database")
    expect(button.textContent).toBe("Cancel")
    button.click()
    expect(resets()).toBe(0)
    expect(confirm.style.display).toBe("none")
    expect(button.textContent).toBe(label)
    expect(button.style.background).toBe(background)
    // Hidden again, the confirming button resets nothing either.
    clock += RESET_CONFIRM_GUARD_MS
    confirm.click()
    expect(resets()).toBe(0)
  }
)
it.each(Object.keys(screens) as (keyof typeof screens)[])(
  "the second click of a double click on the %s screen's reset button cannot confirm",
  async (screen) => {
    await screens[screen]()
    const { button, confirm } = resetButtons()
    button.click()
    clock += RESET_CONFIRM_GUARD_MS - 1
    confirm.click()
    expect(resets()).toBe(0)
    expect(button.textContent).toBe("Cancel")
    clock += 1
    confirm.click()
    expect(resets()).toBe(1)
    expect(confirm.style.display).toBe("none")
    expect(button.textContent).toBe("Resetting...")
    expect(button.disabled).toBe(true)
  }
)
it("Open Folder gives its place to the confirming button and comes back on Cancel", async () => {
  await screens.fatal()
  const { button } = resetButtons()
  const open = document.querySelector<HTMLElement>("#open-db-folder-btn")!
  button.click()
  expect(open.style.display).toBe("none")
  button.click()
  expect(open.style.display).toBe("")
})
it.each(["locked", "access", "storage"])(
  "a %s database gets no reset, only where the database is",
  async (category) => {
    await bridge.api.fatalError("failure", "CoreModule", undefined, category)
    expect(document.querySelector("#reset-db-btn")).toBeNull()
    expect(document.querySelector("#reset-db-confirm-btn")).toBeNull()
    expect(document.body.textContent).not.toMatch(/reset/i)
    expect(document.body.textContent).toContain(
      `/synthetic/runtime${path.sep}gdl_conf.db`
    )
    document.querySelector<HTMLButtonElement>("#open-db-folder-btn")!.click()
    expect(bridge.invoke).toHaveBeenCalledWith(
      "openFolder",
      `/synthetic/runtime${path.sep}gdl_conf.db`
    )
    // The rest of the screen still works without the reset step.
    document.querySelector<HTMLButtonElement>("#restart-btn")!.click()
    expect(bridge.invoke).toHaveBeenCalledWith("relaunch")
  }
)
it.each([
  "corruption",
  "version",
  "downgrade",
  "interrupted_restore",
  "unknown",
  undefined
])("a %s failure still offers the reset", async (category) => {
  await bridge.api.fatalError("failure", "CoreModule", undefined, category)
  expect(document.querySelector("#reset-db-btn")).not.toBeNull()
})

// The version mismatch screen: the update check first, the reset behind a link.
const mismatch = () => ({
  open: document.querySelector<HTMLButtonElement>("#reset-db-btn")!,
  panel: document.querySelector<HTMLElement>("#reset-db-panel")!,
  confirm: document.querySelector<HTMLButtonElement>("#reset-db-confirm-btn")!,
  cancel: document.querySelector<HTMLButtonElement>("#reset-db-cancel-btn")!,
  update: document.querySelector<HTMLElement>("#update-step")!,
  check: document.querySelector<HTMLButtonElement>("#check-updates-btn")!,
  status: document.querySelector<HTMLElement>("#update-status-text")!
})
const settle = () => new Promise((resolve) => setTimeout(resolve, 0))
it("the version mismatch screen leads with the update check and shows the reset only when asked", async () => {
  await bridge.api.backwardsMigrationError()
  const { open, panel, confirm, cancel, update, check } = mismatch()
  expect(check.textContent.trim()).toBe("Retry")
  expect(panel.style.display).toBe("none")
  expect(update.style.display).toBe("flex")
  expect(open.textContent).toBe("Reset database instead")
  expect(document.body.textContent).not.toContain("compatible launcher")
  open.click()
  expect(resets()).toBe(0)
  expect(panel.style.display).toBe("flex")
  expect(update.style.display).toBe("none")
  expect(panel.textContent).toContain("You'll need to sign in again")
  expect(confirm.textContent.trim()).toBe("Reset Database")
  cancel.click()
  expect(resets()).toBe(0)
  expect(panel.style.display).toBe("none")
  expect(update.style.display).toBe("flex")
  // Hidden again, the confirming button resets nothing.
  clock += RESET_CONFIRM_GUARD_MS
  confirm.click()
  expect(resets()).toBe(0)
})
it("the second click of a double click on the mismatch screen's reset link cannot confirm", async () => {
  await bridge.api.backwardsMigrationError()
  const { open, confirm, cancel } = mismatch()
  bridge.invoke.mockReset()
  bridge.invoke.mockResolvedValue({ ok: true, recoveryPath: "/synthetic" })
  open.click()
  clock += RESET_CONFIRM_GUARD_MS - 1
  confirm.click()
  await settle()
  expect(resets()).toBe(0)
  clock += 1
  confirm.click()
  expect(confirm.textContent).toBe("Resetting...")
  expect(cancel.disabled).toBe(true)
  await settle()
  expect(resets()).toBe(1)
  // It worked: both stay busy until the restart.
  expect(confirm.disabled).toBe(true)
  expect(cancel.disabled).toBe(true)
  cancel.click()
  expect(mismatch().panel.style.display).toBe("flex")
})
it("a reset that fails on the mismatch screen says why and can be retried or cancelled", async () => {
  await bridge.api.backwardsMigrationError()
  const { confirm, cancel, panel } = mismatch()
  bridge.invoke.mockReset()
  bridge.invoke.mockRejectedValue(Error("controlled refusal"))
  confirmReset()
  await settle()
  expect(confirm.textContent).toBe("Retry Reset")
  expect(confirm.disabled).toBe(false)
  expect(cancel.disabled).toBe(false)
  expect(document.querySelector("#reset-db-error")!.textContent).toContain(
    "Reset did not complete"
  )
  bridge.invoke.mockResolvedValue({ ok: true, recoveryPath: "/synthetic" })
  confirm.click()
  await settle()
  expect(resets()).toBe(2)
  expect(confirm.disabled).toBe(true)
  expect(panel.style.display).toBe("flex")
})
it("Get help on Discord opens the Discord invite from the mismatch screen", async () => {
  await bridge.api.backwardsMigrationError()
  document.querySelector<HTMLButtonElement>("#discord-btn")!.click()
  expect(bridge.invoke).toHaveBeenCalledWith(
    "openExternalLink",
    "https://discord.gdlauncher.com"
  )
})

// The update check both screens share.
const updateScreens = {
  fatal: () => bridge.api.fatalError("failure", "CoreModule"),
  mismatch: () => bridge.api.backwardsMigrationError()
}
const state = (revision: number, value: Record<string, unknown>) => ({
  revision,
  updateInfo: null,
  progress: 0,
  error: null,
  ...value
})
/** Main's update state as the screen is shown, and its answer to a check. */
function updater(initial: unknown, checked?: unknown) {
  bridge.invoke.mockImplementation(async (name: string) =>
    name === "getRuntimePath"
      ? "/synthetic/runtime"
      : name === "getUpdateState"
        ? initial
        : name === "checkForUpdatesFromRecovery"
          ? checked
          : undefined
  )
}
const pushed = (value: unknown) => {
  for (const [channel, listener] of bridge.on.mock.calls)
    if (channel === "gd-update-state-changed") listener({}, value)
}
const calls = (name: string) =>
  bridge.invoke.mock.calls.filter((call) => call[0] === name)
it.each(Object.keys(updateScreens) as (keyof typeof updateScreens)[])(
  "the %s screen checks by itself, shows the download and applies it on a click",
  async (screen) => {
    updater(state(1, { state: "idle" }), {
      status: "checking",
      currentState: state(2, {
        state: "downloading",
        updateInfo: { version: "2.1.0" },
        progress: 43.4
      })
    })
    await updateScreens[screen]()
    const { check, status } = mismatch()
    const bar = document.querySelector<HTMLElement>("#update-progress-bar")!
    const progress = document.querySelector<HTMLElement>(
      "#update-progress-container"
    )!
    expect(calls("checkForUpdatesFromRecovery")).toHaveLength(1)
    expect(status.textContent).toBe("Downloading v2.1.0")
    expect(check.disabled).toBe(true)
    expect(progress.style.display).toBe("block")
    expect(bar.style.width).toBe("43%")
    // An older state arriving late changes nothing.
    pushed(state(1, { state: "idle" }))
    expect(status.textContent).toBe("Downloading v2.1.0")
    pushed(
      state(3, {
        state: "downloaded",
        updateInfo: { version: "2.1.0" },
        readyUpdate: { id: 7, version: "2.1.0" }
      })
    )
    expect(status.textContent).toBe("Update v2.1.0 ready")
    expect(check.textContent).toBe("Apply & Restart")
    expect(check.disabled).toBe(false)
    expect(progress.style.display).toBe("none")
    check.click()
    await settle()
    expect(bridge.invoke).toHaveBeenCalledWith("installUpdate", 7)
    expect(calls("checkForUpdatesFromRecovery")).toHaveLength(1)
  }
)
it.each(Object.keys(updateScreens) as (keyof typeof updateScreens)[])(
  "the %s screen offers the download link when this install cannot update itself",
  async (screen) => {
    const writeText = vi.fn(async () => {})
    vi.stubGlobal("navigator", { clipboard: { writeText } })
    updater(
      state(4, {
        state: "manual-update",
        updateInfo: { version: "2.1.0", downloadUrl: "https://synthetic/x.deb" }
      })
    )
    await updateScreens[screen]()
    const { check, status } = mismatch()
    expect(calls("checkForUpdatesFromRecovery")).toHaveLength(0)
    expect(status.textContent).toContain("Update v2.1.0 available")
    expect(check.textContent).toBe("Copy Link")
    check.click()
    await settle()
    expect(writeText).toHaveBeenCalledWith("https://synthetic/x.deb")
  }
)
it("the mismatch screen says what to do when the channel has nothing newer, and checks again on a click", async () => {
  updater(state(1, { state: "no-update" }), {
    status: "no-update",
    currentState: state(2, { state: "no-update" })
  })
  await bridge.api.backwardsMigrationError()
  const { check, status } = mismatch()
  expect(status.textContent).toBe(
    "No newer version found. Install the version you used before."
  )
  expect(status.style.whiteSpace).toBe("normal")
  expect(check.textContent).toBe("Check for Updates")
  check.click()
  await settle()
  expect(calls("checkForUpdatesFromRecovery")).toHaveLength(1)
})
it("the fatal screen keeps its Discord wording when the channel has nothing newer", async () => {
  updater(state(1, { state: "no-update" }))
  await bridge.api.fatalError("failure", "CoreModule")
  const { status } = mismatch()
  expect(status.textContent).toContain("report this issue on Discord")
  document.querySelector<HTMLElement>("#discord-link-inline")!.click()
  expect(bridge.invoke).toHaveBeenCalledWith(
    "openExternalLink",
    "https://discord.gdlauncher.com"
  )
})
it.each(Object.keys(updateScreens) as (keyof typeof updateScreens)[])(
  "the %s screen reports a failed check and a failed install, and retries",
  async (screen) => {
    updater(state(1, { state: "idle" }))
    bridge.invoke.mockImplementation(async (name: string) => {
      if (name === "getRuntimePath") return "/synthetic/runtime"
      if (name === "getUpdateState") return state(1, { state: "idle" })
      throw Error("controlled")
    })
    await updateScreens[screen]()
    const { check, status } = mismatch()
    expect(status.textContent).toBe("Failed to check for updates")
    expect(check.textContent).toBe("Retry")
    pushed(
      state(5, {
        state: "downloaded",
        updateInfo: { version: "2.1.0" },
        readyUpdate: { id: 9, version: "2.1.0" }
      })
    )
    check.click()
    await settle()
    expect(bridge.invoke).toHaveBeenCalledWith("installUpdate", 9)
    expect(status.textContent).toBe("controlled")
    // The ready update is still owned, so Retry installs it again.
    check.click()
    await settle()
    expect(calls("installUpdate")).toHaveLength(2)
  }
)
