// @vitest-environment jsdom
import { it, expect, vi, beforeEach } from "vitest"
import path from "node:path"
const bridge = vi.hoisted(() => ({
  api: {} as Record<string, any>,
  invoke: vi.fn(),
  on: vi.fn()
}))
vi.mock("electron", () => ({
  contextBridge: {
    exposeInMainWorld: (name: string, value: any) => {
      bridge.api[name] = value
    }
  },
  ipcRenderer: { invoke: bridge.invoke, on: bridge.on }
}))
vi.mock("./utils", () => ({ domReady: async () => true }))
async function ready() {
  document.body.innerHTML = '<div id="appFatalCrash"></div>'
  vi.stubGlobal("__APP_VERSION__", "synthetic")
  bridge.invoke.mockReset()
  bridge.on.mockReset()
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
  expect(document.body.textContent).toContain("server registrations")
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
    document.querySelector<HTMLButtonElement>("#reset-db-btn")!.click()
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
it.each(["fatal", "backwards"])(
  "%s rejected reset IPC is safe and retryable; success remains busy",
  async (screen) => {
    await (screen === "fatal"
      ? bridge.api.fatalError("failure", "CoreModule")
      : bridge.api.backwardsMigrationError())
    bridge.invoke.mockRejectedValue(Error("controlled refusal"))
    const button = document.querySelector<HTMLButtonElement>("#reset-db-btn")!
    button.click()
    await new Promise((resolve) => setTimeout(resolve, 0))
    expect(button.disabled).toBe(false)
    expect(button.textContent).toBe("Retry Reset")
    bridge.invoke.mockResolvedValue({
      ok: true,
      recoveryPath: "/synthetic/retained"
    })
    button.click()
    await new Promise((resolve) => setTimeout(resolve, 0))
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
