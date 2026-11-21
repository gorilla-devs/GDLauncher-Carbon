import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"

const io = vi.hoisted(() => ({
  replace: vi.fn(),
  locate: vi.fn(),
  close: vi.fn(),
  open: vi.fn(),
  recycleBin: true
}))
vi.mock("@gd/ui", () => ({
  toast: { error: vi.fn() },
  Button: (p: any) => (
    <button
      data-testid={p["data-testid"]}
      disabled={p.disabled}
      onClick={p.onClick}
    >
      {p.children}
    </button>
  ),
  Checkbox: (p: any) => (
    <label>
      <input
        type="checkbox"
        aria-label={p.ariaLabel}
        checked={p.checked}
        onChange={(event) => p.onChange?.(event.currentTarget.checked)}
      />
      {p.children}
    </label>
  )
}))
const translate = (key: string, options?: Record<string, unknown>) => {
  if (!options) return key
  const { interpolation: _, ...rest } = options
  return `${key} ${JSON.stringify(rest)}`
}
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => <span>{translate(p.key, p.options)}</span>,
  useTransContext: () => [translate]
}))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ closeModal: io.close })
}))
vi.mock("@/managers/ModalsManager/ModalLayout", () => ({
  default: (p: any) => <div>{p.children}</div>
}))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    createQuery: () => ({
      get data() {
        return { deletionThroughRecycleBin: io.recycleBin }
      }
    }),
    createMutation: (options: () => { mutationKey: string[] }) => ({
      isPending: false,
      mutateAsync:
        options().mutationKey[0] === "instance.usePackVersions"
          ? io.replace
          : io.locate
    })
  }
}))

import KeptPackFiles from "./index"

const FILES = ["config/a.toml", "config/jei/b.ini", "options.cfg"]
let dispose: (() => void) | undefined
let root: HTMLElement
const flush = async () => {
  for (let i = 0; i < 8; i++) await Promise.resolve()
}
const mount = () => {
  root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => (
      <KeptPackFiles
        data={{ id: 5, name: "ATM 10", files: FILES }}
        title="Kept files"
      />
    ),
    root
  )
}
const rows = () =>
  [...root.querySelectorAll('[data-testid="kept-file-row"]')].map(
    (row) => row.textContent
  )
const useThePacks = () =>
  root.querySelector<HTMLButtonElement>('[data-testid="kept-files-use-pack"]')!

beforeEach(() => {
  vi.clearAllMocks()
  io.recycleBin = true
  window.openFolder = io.open
})
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
})

it("lists each file by name and folder, and takes the modpack's version of all of them with one button", async () => {
  io.replace.mockResolvedValue(FILES.map((path) => ({ path, error: null })))
  mount()
  expect(root.textContent).toContain(
    'instances:_trn_kept_files_intro {"name":"ATM 10"}'
  )
  expect(root.textContent).toContain(
    'instances:_trn_kept_files_count {"count":3}'
  )
  expect(rows()).toEqual(["a.tomlconfig", "b.iniconfig/jei", "options.cfg"])
  // No checkboxes until the player asks to choose.
  expect(root.querySelector('input[type="checkbox"]')).toBeNull()
  expect(root.textContent).toContain("instances:_trn_kept_files_recycle_bin")

  useThePacks().click()
  await flush()
  expect(io.replace).toHaveBeenCalledWith({ instance: 5, paths: FILES })
  expect(io.close).toHaveBeenCalledTimes(1)
})

it("replaces only the files left ticked once the player chooses", async () => {
  io.replace.mockResolvedValue([{ path: "config/a.toml", error: null }])
  mount()
  root
    .querySelector<HTMLButtonElement>('[data-testid="kept-files-choose"]')!
    .click()
  const box = (path: string) =>
    root.querySelector<HTMLInputElement>(`input[aria-label="${path}"]`)!
  expect(box("config/a.toml").checked).toBe(true)
  box("config/jei/b.ini").click()
  box("options.cfg").click()
  expect(root.textContent).toContain(
    'instances:_trn_kept_files_selected {"selected":1,"total":3}'
  )
  useThePacks().click()
  await flush()
  expect(io.replace).toHaveBeenCalledWith({
    instance: 5,
    paths: ["config/a.toml"]
  })
  expect(io.close).toHaveBeenCalledTimes(1)

  // With nothing ticked there is nothing to replace.
  box("config/a.toml").click()
  expect(useThePacks().disabled).toBe(true)
})

it("stays open with the files that could not be replaced, and says why", async () => {
  io.replace.mockResolvedValue([
    { path: "config/a.toml", error: null },
    { path: "config/jei/b.ini", error: "its folder is a link" },
    { path: "options.cfg", error: "it is no longer a file" }
  ])
  mount()
  useThePacks().click()
  await flush()
  expect(io.close).not.toHaveBeenCalled()
  expect(rows()).toEqual(["b.iniconfig/jei", "options.cfg"])
  expect(
    root.querySelector('[data-testid="kept-files-failure"]')?.textContent
  ).toBe(
    'instances:_trn_kept_files_failed {"count":2,"reason":"its folder is a link"}'
  )

  // A refusal of the whole request is shown the same way.
  io.replace.mockRejectedValue(new Error("the instance is running"))
  useThePacks().click()
  await flush()
  expect(
    root.querySelector('[data-testid="kept-files-failure"]')?.textContent
  ).toContain("the instance is running")
  expect(io.close).not.toHaveBeenCalled()
})

it("shows a file in its folder, and promises the Recycle Bin only when it is on", async () => {
  io.recycleBin = false
  io.locate.mockResolvedValue("/instances/atm/instance/config/jei/b.ini")
  mount()
  expect(root.textContent).not.toContain("_trn_kept_files_recycle_bin")
  root
    .querySelectorAll<HTMLElement>('[data-testid="kept-file-show"]')[1]
    .click()
  await flush()
  expect(io.locate).toHaveBeenCalledWith({
    instance: 5,
    path: "config/jei/b.ini"
  })
  expect(io.open).toHaveBeenCalledWith(
    "/instances/atm/instance/config/jei/b.ini"
  )
})
