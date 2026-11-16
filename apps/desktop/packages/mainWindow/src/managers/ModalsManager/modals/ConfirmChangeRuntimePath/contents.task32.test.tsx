import { afterEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import fss from "node:fs"
import path from "node:path"
import type { RuntimePathChange } from "../../../../../../main/utils/runtimeRelocation"
import ConfirmChangeRuntimePath from "."

const c = vi.hoisted(() => {
  window.changeRuntimePathProgress = vi.fn()
  return { close: vi.fn() }
})
vi.mock("@gd/ui", async () => import("@/task32.fixture"))
vi.mock("@gd/i18n", () => ({
  useTransContext: () => [(key: string) => key],
  Trans: (p: any) => (
    <span>
      {p.key}
      {p.options?.number !== undefined ? `=${p.options.number}` : ""}
    </span>
  )
}))
vi.mock("../..", () => ({ useModal: () => ({ closeModal: c.close }) }))
vi.mock("../../ModalLayout", () => ({
  default: (p: any) => (
    <div>
      <h2>{p.title}</h2>
      {p.children}
    </div>
  )
}))

let dispose: (() => void) | undefined
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
  c.close.mockClear()
})
const flush = async () => {
  for (let i = 0; i < 10; i++) await Promise.resolve()
}
const byTestId = (id: string) =>
  document.querySelector<HTMLElement>(`[data-testid="${id}"]`)
const confirmKey = (switchOnly: boolean) =>
  switchOnly
    ? "java:_trn_runtime_switch.confirm"
    : "settings:_trn_confirm_change_confirm_button"
const confirm = (switchOnly: boolean) =>
  Array.from(document.querySelectorAll("button")).find((button) =>
    button.textContent?.includes(confirmKey(switchOnly))
  )!

async function open(
  switchOnly: boolean,
  change: RuntimePathChange | Promise<RuntimePathChange> | Error
) {
  const changeRuntimePath = vi.fn(async () => ({
    type: "relaunching" as const
  }))
  Object.assign(window, {
    getRuntimePath: async () => "/old",
    describeRuntimePathChange: vi.fn(async () => {
      if (change instanceof Error) throw change
      return change
    }),
    changeRuntimePath
  })
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => (
      <ConfirmChangeRuntimePath
        title="registry title"
        data={{ runtimePath: "/new", isTargetFolderAlreadyUsed: switchOnly }}
      />
    ),
    root
  )
  await flush()
  return { changeRuntimePath }
}
const nothing = { found: null, unfamiliar: [], unfamiliarCount: 0 }

it("says what a switch opens, and that nothing is moved", async () => {
  const f = await open(true, {
    ...nothing,
    found: { instances: 12, servers: 2, database: true },
    // What the old folder holds is no business of a switch.
    unfamiliar: ["Steam"],
    unfamiliarCount: 1
  })
  const text = document.body.textContent
  expect(document.querySelector("h2")!.textContent).toBe(
    "java:_trn_runtime_switch.title"
  )
  expect(text).toContain("java:_trn_runtime_switch.text")
  expect(text).toContain("java:_trn_runtime_switch.found_instances=12")
  expect(text).toContain("java:_trn_runtime_switch.found_servers=2")
  expect(text).toContain("java:_trn_runtime_switch.found_database")
  expect(text).toContain("java:_trn_runtime_switch.nothing_moved")
  expect(byTestId("runtime-switch-no-database")).toBeNull()
  expect(byTestId("runtime-unfamiliar")).toBeNull()
  confirm(true).click()
  await flush()
  expect(f.changeRuntimePath).toHaveBeenCalledWith("/new", true, false, false)
})

it("says what a data folder without its database brings back and what it does not", async () => {
  await open(true, {
    ...nothing,
    found: { instances: 3, servers: 0, database: false }
  })
  const text = document.body.textContent
  expect(text).toContain("java:_trn_runtime_switch.found_instances=3")
  expect(text).toContain("java:_trn_runtime_switch.found_servers=0")
  expect(byTestId("runtime-switch-no-database")!.textContent).toContain(
    "java:_trn_runtime_switch.no_database"
  )
  expect(text).not.toContain("java:_trn_runtime_switch.found_database")
})

it("keeps the move dialog as it is when the old folder holds only the launcher's data", async () => {
  const f = await open(false, nothing)
  const text = document.body.textContent
  expect(document.querySelector("h2")!.textContent).toBe("registry title")
  expect(text).toContain("java:_trn_confirm_change_runtime_path_text")
  expect(byTestId("runtime-unfamiliar")).toBeNull()
  expect(byTestId("runtime-switch-notes")).toBeNull()
  confirm(false).click()
  await flush()
  expect(f.changeRuntimePath).toHaveBeenCalledWith("/new", false, false, false)
})

it("lists what is not the launcher's and leaves it unless the user chooses to move it", async () => {
  const f = await open(false, {
    found: null,
    unfamiliar: ["Epic", "notes.txt", "Steam"],
    unfamiliarCount: 3
  })
  const section = byTestId("runtime-unfamiliar")!
  expect(section.textContent).toContain("java:_trn_runtime_unfamiliar.title=3")
  expect(
    Array.from(section.querySelectorAll("li")).map((li) => li.textContent)
  ).toEqual(["Epic", "notes.txt", "Steam"])
  const radios = Array.from(
    section.querySelectorAll<HTMLInputElement>('input[type="radio"]')
  )
  expect(radios.map((radio) => radio.checked)).toEqual([true, false])
  confirm(false).click()
  await flush()
  expect(f.changeRuntimePath).toHaveBeenLastCalledWith(
    "/new",
    false,
    false,
    false
  )

  radios[1].click()
  await flush()
  expect(radios.map((radio) => radio.checked)).toEqual([false, true])
  confirm(false).click()
  await flush()
  expect(f.changeRuntimePath).toHaveBeenLastCalledWith(
    "/new",
    false,
    true,
    false
  )

  radios[0].click()
  await flush()
  confirm(false).click()
  await flush()
  expect(f.changeRuntimePath).toHaveBeenLastCalledWith(
    "/new",
    false,
    false,
    false
  )
})

it("says one item without a plural and counts the ones it does not list", async () => {
  await open(false, {
    found: null,
    unfamiliar: ["Steam"],
    unfamiliarCount: 1
  })
  expect(byTestId("runtime-unfamiliar")!.textContent).toContain(
    "java:_trn_runtime_unfamiliar.title_single"
  )
  dispose?.()
  document.body.textContent = ""
  await open(false, {
    found: null,
    unfamiliar: ["a", "b"],
    unfamiliarCount: 7
  })
  const text = byTestId("runtime-unfamiliar")!.textContent
  expect(text).toContain("java:_trn_runtime_unfamiliar.title=7")
  expect(text).toContain("java:_trn_runtime_unfamiliar.more=5")
})

it("cannot be confirmed before it knows what the old folder holds", async () => {
  let resolve!: (value: RuntimePathChange) => void
  const f = await open(
    false,
    new Promise<RuntimePathChange>((done) => (resolve = done))
  )
  expect(confirm(false).disabled).toBe(true)
  confirm(false).click()
  await flush()
  expect(f.changeRuntimePath).not.toHaveBeenCalled()
  resolve({ found: null, unfamiliar: ["Steam"], unfamiliarCount: 1 })
  await flush()
  expect(confirm(false).disabled).toBe(false)
  expect(byTestId("runtime-unfamiliar")).not.toBeNull()
})

it("still moves, leaving everything unfamiliar, when the folders cannot be described", async () => {
  const f = await open(false, new Error("EACCES"))
  expect(byTestId("runtime-unfamiliar")).toBeNull()
  expect(confirm(false).disabled).toBe(false)
  confirm(false).click()
  await flush()
  expect(f.changeRuntimePath).toHaveBeenCalledWith("/new", false, false, false)
})

it("has every new string in every shipped language", () => {
  const locales = path.resolve("../../packages/i18n/locale")
  const english = JSON.parse(
    fss.readFileSync(path.join(locales, "english", "java.json"), "utf8")
  )
  const keys = Object.keys(english).filter((key) =>
    /^_trn_runtime_(switch|unfamiliar)\./.test(key)
  )
  expect(keys).toHaveLength(17)
  for (const language of fss.readdirSync(locales)) {
    if (!fss.statSync(path.join(locales, language)).isDirectory()) continue
    const java = JSON.parse(
      fss.readFileSync(path.join(locales, language, "java.json"), "utf8")
    )
    for (const key of keys) {
      expect(java[key], `${language} ${key}`).toMatch(/\S/)
      // A count the English names is named in every language.
      if (english[key].includes("{{number}}"))
        expect(java[key], `${language} ${key}`).toContain("{{number}}")
    }
  }
})
