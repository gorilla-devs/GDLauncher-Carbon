import { afterEach, describe, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import fss from "node:fs"
import path from "node:path"
import RuntimePath from "./RuntimePath"

const c = vi.hoisted(() => ({
  open: vi.fn(),
  apply: undefined as any,
  moving: false
}))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ openModal: c.open })
}))
vi.mock(
  "@/managers/ModalsManager/modals/ConfirmChangeRuntimePath/state",
  () => ({ isChangingRuntimePath: () => c.moving })
)
vi.mock("@gd/i18n", () => ({ Trans: (p: any) => <span>{p.key}</span> }))
vi.mock("@gd/ui", () => ({
  Button: (p: any) => {
    if (p.type === "primary") c.apply = p
    return (
      <button disabled={p.disabled} onClick={p.onClick}>
        {p.children}
      </button>
    )
  },
  Input: (p: any) => (
    <div>
      <input value={p.value} onInput={p.onInput} />
      {p.icon}
    </div>
  ),
  Tooltip: (p: any) => <div>{p.children}</div>,
  TooltipTrigger: (p: any) => <div>{p.children}</div>,
  TooltipContent: (p: any) => <div>{p.children}</div>
}))
let dispose: () => void
afterEach(() => {
  dispose?.()
  document.body.textContent = ""
  c.open.mockClear()
  c.moving = false
  vi.restoreAllMocks()
})
const flush = async () => {
  for (let i = 0; i < 8; i++) await Promise.resolve()
}
it("GPT-0261 admits only the settled current path in both presentation and direct handler", async () => {
  const pending = new Map<
    string,
    { resolve: (v: any) => void; reject: (v: any) => void }
  >()
  Object.assign(window, {
    getRuntimePath: async () => "C",
    getInitialRuntimePath: async () => "initial",
    validateRuntimePath: vi.fn(
      (path: string) =>
        new Promise((resolve, reject) => pending.set(path, { resolve, reject }))
    ),
    openFileDialog: async () => ({ canceled: false, filePaths: ["dialog"] })
  })
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(() => <RuntimePath />, root)
  await flush()
  const input = root.querySelector("input")!
  const select = (path: string) => {
    input.value = path
    input.dispatchEvent(new Event("input", { bubbles: true }))
  }
  select("A")
  pending.get("A")!.resolve("valid")
  await flush()
  expect(c.apply.disabled).toBe(false)
  select("B")
  c.apply.onClick()
  expect(c.open).not.toHaveBeenCalled()
  expect(c.apply.disabled).toBe(true)
  pending.get("B")!.resolve("potentially_valid")
  await flush()
  c.apply.onClick()
  expect(c.open).toHaveBeenLastCalledWith(
    { name: "ConfirmChangeRuntimePath" },
    { runtimePath: "B", isTargetFolderAlreadyUsed: true }
  )
  c.open.mockClear()
  select("failed")
  pending.get("failed")!.reject(new Error("synthetic validation"))
  await flush()
  c.apply.onClick()
  expect(c.open).not.toHaveBeenCalled()
  root.querySelector<HTMLElement>(".i-hugeicons\\:folder-01")!.click()
  await flush()
  c.apply.onClick()
  expect(c.open).not.toHaveBeenCalled()
  pending.get("dialog")!.resolve("valid")
  await flush()
  c.apply.onClick()
  expect(c.open).toHaveBeenLastCalledWith(expect.anything(), {
    runtimePath: "dialog",
    isTargetFolderAlreadyUsed: false
  })
  c.open.mockClear()
  select("C")
  pending.get("C")!.resolve("valid")
  await flush()
  c.apply.onClick()
  expect(c.open).not.toHaveBeenCalled()
})

it("names an unfinished move instead of calling the folder invalid", async () => {
  Object.assign(window, {
    getRuntimePath: async () => "C",
    getInitialRuntimePath: async () => "initial",
    validateRuntimePath: vi.fn(async (path: string) =>
      path === "partial"
        ? "incomplete_migration"
        : path === "legacy"
          ? "other_incomplete_migration"
          : "invalid"
    ),
    openFileDialog: async () => ({ canceled: true, filePaths: [] })
  })
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(() => <RuntimePath />, root)
  await flush()
  const input = root.querySelector("input")!
  const select = async (value: string) => {
    input.value = value
    input.dispatchEvent(new Event("input", { bubbles: true }))
    await flush()
  }
  await select("partial")
  expect(c.apply.disabled).toBe(true)
  c.apply.onClick()
  expect(c.open).not.toHaveBeenCalled()
  expect(root.textContent).toContain("ui:_trn_tooltip.rtp_incomplete_migration")
  expect(root.textContent).not.toContain("ui:_trn_tooltip.rtp_not_valid")
  // A marked folder that may have been someone's data folder gets cautious advice.
  await select("legacy")
  expect(c.apply.disabled).toBe(true)
  expect(root.textContent).toContain(
    "ui:_trn_tooltip.rtp_other_incomplete_migration"
  )
  expect(root.textContent).not.toContain(
    "ui:_trn_tooltip.rtp_incomplete_migration"
  )
  await select("elsewhere")
  expect(root.textContent).toContain("ui:_trn_tooltip.rtp_not_valid")
  expect(root.textContent).not.toContain(
    "ui:_trn_tooltip.rtp_incomplete_migration"
  )
  // The tooltip is translated in every shipped language.
  const locales = path.resolve("../../packages/i18n/locale")
  const languages = fss
    .readdirSync(locales, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
  expect(languages).toHaveLength(7)
  for (const language of languages) {
    const ui = JSON.parse(
      fss.readFileSync(path.join(locales, language, "ui.json"), "utf8")
    )
    expect(ui["_trn_tooltip.rtp_incomplete_migration"], language).toMatch(/\S/)
    expect(ui["_trn_tooltip.rtp_other_incomplete_migration"], language).toMatch(
      /\.gdl_migration_in_progress/
    )
    // The file is hidden in Finder and Linux file managers: say how to show it.
    expect(ui["_trn_tooltip.rtp_other_incomplete_migration"], language).toMatch(
      /Cmd\+\S+\+\..*Ctrl\+H/
    )
  }
})

describe("a folder the user frees outside the launcher", () => {
  async function mount() {
    const disk = { deleted: false }
    const validate = vi.fn(async (path: string) =>
      path === "/d/GDL"
        ? disk.deleted
          ? "valid"
          : "incomplete_migration"
        : path === "/e/Empty"
          ? "valid"
          : "invalid"
    )
    let picked = "/d/GDL"
    Object.assign(window, {
      getRuntimePath: async () => "C",
      getInitialRuntimePath: async () => "initial",
      validateRuntimePath: validate,
      openFileDialog: async () => ({ canceled: false, filePaths: [picked] })
    })
    const root = document.createElement("div")
    document.body.append(root)
    dispose = render(() => <RuntimePath />, root)
    await flush()
    const pick = async (path = "/d/GDL") => {
      picked = path
      root.querySelector<HTMLElement>(".i-hugeicons\\:folder-01")!.click()
      await flush()
    }
    const checks = (path: string) =>
      validate.mock.calls.filter(([value]) => value === path).length
    return { disk, root, pick, checks }
  }

  it("checks the same folder again when it is picked again", async () => {
    const m = await mount()
    await m.pick()
    expect(c.apply.disabled).toBe(true)
    expect(m.root.textContent).toContain(
      "ui:_trn_tooltip.rtp_incomplete_migration"
    )
    // The user deletes the partial copy in Explorer or Finder, then picks it again.
    m.disk.deleted = true
    await m.pick()
    expect(m.checks("/d/GDL")).toBe(2)
    expect(c.apply.disabled).toBe(false)
    expect(m.root.textContent).toContain("ui:_trn_tooltip.apply_and_restart")
    expect(m.root.textContent).not.toContain(
      "ui:_trn_tooltip.rtp_incomplete_migration"
    )
    c.apply.onClick()
    expect(c.open).toHaveBeenLastCalledWith(expect.anything(), {
      runtimePath: "/d/GDL",
      isTargetFolderAlreadyUsed: false
    })
  })

  it("checks a refused folder again when the window regains focus", async () => {
    const m = await mount()
    await m.pick()
    expect(c.apply.disabled).toBe(true)
    m.disk.deleted = true
    window.dispatchEvent(new Event("focus"))
    await flush()
    expect(m.checks("/d/GDL")).toBe(2)
    expect(c.apply.disabled).toBe(false)
    expect(m.root.textContent).toContain("ui:_trn_tooltip.apply_and_restart")
  })

  it("leaves an accepted folder and the current one alone on focus", async () => {
    const m = await mount()
    // The unchanged current folder is never applicable; nothing to re-check.
    window.dispatchEvent(new Event("focus"))
    await flush()
    expect(m.checks("C")).toBe(1)
    await m.pick("/e/Empty")
    expect(c.apply.disabled).toBe(false)
    // A click that focuses the window must still apply, so no refresh starts.
    window.dispatchEvent(new Event("focus"))
    expect(c.apply.disabled).toBe(false)
    c.apply.onClick()
    expect(c.open).toHaveBeenLastCalledWith(expect.anything(), {
      runtimePath: "/e/Empty",
      isTargetFolderAlreadyUsed: false
    })
    await flush()
    expect(m.checks("/e/Empty")).toBe(1)
  })

  it("stops listening for focus when Settings closes", async () => {
    const m = await mount()
    await m.pick()
    dispose()
    dispose = () => {}
    m.disk.deleted = true
    window.dispatchEvent(new Event("focus"))
    await flush()
    expect(m.checks("/d/GDL")).toBe(1)
  })
})

it("says why a folder is refused instead of calling it not a folder or not empty", async () => {
  const reasons = {
    "Z:\\": ["filesystem_root", "rtp_drive_root"],
    "\\\\nas\\share\\GDL": ["network_path", "rtp_network_path"],
    relative: ["not_absolute", "rtp_not_absolute"],
    "/usr/gdl": ["system_dir", "rtp_system_dir"],
    "C/nested": ["nested", "rtp_nested"],
    "/opt/GDLauncher/data": ["install_overlap", "rtp_install_overlap"],
    "/run/media/me/USB/gdl": ["removable_not_mounted", "rtp_not_mounted"],
    "/home/me/file": ["not_a_folder", "rtp_not_a_folder"],
    "/locked/gdl": ["not_writable", "rtp_not_writable"]
  } as Record<string, [string, string]>
  Object.assign(window, {
    getRuntimePath: async () => "C",
    getInitialRuntimePath: async () => "initial",
    validateRuntimePath: vi.fn(
      async (path: string) => reasons[path]?.[0] ?? "invalid"
    ),
    openFileDialog: async () => ({ canceled: true, filePaths: [] })
  })
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(() => <RuntimePath />, root)
  await flush()
  const input = root.querySelector("input")!
  for (const [value, [, key]] of Object.entries(reasons)) {
    input.value = value
    input.dispatchEvent(new Event("input", { bubbles: true }))
    await flush()
    expect(c.apply.disabled, value).toBe(true)
    expect(root.textContent, value).toContain(`ui:_trn_tooltip.${key}`)
    expect(root.textContent, value).not.toContain(
      "ui:_trn_tooltip.rtp_not_valid"
    )
  }
  // A refusal without a reason of its own keeps the general tooltip.
  input.value = "unreadable"
  input.dispatchEvent(new Event("input", { bubbles: true }))
  await flush()
  expect(root.textContent).toContain("ui:_trn_tooltip.rtp_not_valid")
  // Every reason is translated in every shipped language.
  const locales = path.resolve("../../packages/i18n/locale")
  for (const language of fss.readdirSync(locales)) {
    if (!fss.statSync(path.join(locales, language)).isDirectory()) continue
    const ui = JSON.parse(
      fss.readFileSync(path.join(locales, language, "ui.json"), "utf8")
    )
    for (const [, key] of Object.values(reasons))
      expect(ui[`_trn_tooltip.${key}`], `${language} ${key}`).toMatch(/\S/)
  }
})
