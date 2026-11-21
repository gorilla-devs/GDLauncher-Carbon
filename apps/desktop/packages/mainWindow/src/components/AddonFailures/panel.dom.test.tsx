import { afterEach, beforeAll, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import i18next, { type i18n as I18n } from "i18next"
import { createI18nOptions } from "../../../../../../../packages/i18n/index"
import content from "../../../../../../../packages/i18n/locale/english/content.json"
import general from "../../../../../../../packages/i18n/locale/english/general.json"
import tasks from "../../../../../../../packages/i18n/locale/english/tasks.json"
import ruContent from "../../../../../../../packages/i18n/locale/russian/content.json"
import ruGeneral from "../../../../../../../packages/i18n/locale/russian/general.json"
import ruTasks from "../../../../../../../packages/i18n/locale/russian/tasks.json"
import AddonFailures from "./index"

// The panel renders through the shipped bundles, so a title or label that
// leaks English, HTML entities or a duplicate label shows up here.
const c = vi.hoisted(() => ({
  i18n: undefined as unknown as I18n,
  tasks: [] as unknown[],
  options: undefined as Record<string, unknown> | undefined,
  query: vi.fn(),
  mutation: vi.fn(),
  refetch: vi.fn(),
  isError: false
}))
vi.mock("@gd/i18n", () => ({
  useTransContext: () => [
    (key: string, options?: Record<string, unknown>) => c.i18n.t(key, options)
  ]
}))
vi.mock("@gd/ui", () => ({
  Button: (p: any) => (
    <button disabled={p.disabled} onClick={p.onClick}>
      {p.children}
    </button>
  )
}))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    useContext: () => ({ client: { query: c.query, mutation: c.mutation } }),
    createQuery: (options: () => Record<string, unknown>) => {
      c.options = options()
      return {
        get data() {
          return c.tasks
        },
        get isError() {
          return c.isError
        },
        refetch: c.refetch
      }
    }
  }
}))

const failed = (id: number, translation: string, args: object) => ({
  id,
  name: { translation, args },
  progress: {
    type: "Failed",
    value: {
      cause: [{ display: "Checksum mismatch", debug: "debug" }],
      backtrace: ""
    }
  }
})

let dispose: (() => void) | undefined
beforeAll(async () => {
  c.i18n = i18next.createInstance()
  await c.i18n.init(
    createI18nOptions(
      "russian",
      { content, general, tasks },
      { content: ruContent, general: ruGeneral, tasks: ruTasks }
    )
  )
})
afterEach(() => {
  dispose?.()
  document.body.textContent = ""
  c.query.mockReset()
  c.mutation.mockReset()
  c.refetch.mockReset()
  c.isError = false
})

it("titles failures with verbatim names and keeps Close and Remove apart", async () => {
  c.tasks = [
    failed(1, "ServerTaskInstallMod", {
      // The backend's fixed English label, never shown to the player.
      mod_name: "Server addon and required dependencies",
      server_name: "Tom & Jerry"
    }),
    failed(2, "InstanceTaskStageDatapack", {
      mod_name: "Dungeons & Taverns",
      instance_name: "Bob's World"
    })
  ]
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(() => <AddonFailures />, root)
  root.querySelector("button")!.click()
  await Promise.resolve()

  const titles = [...root.querySelectorAll("article strong")].map(
    (title) => title.textContent
  )
  expect(titles).toEqual([
    "Установка дополнений на Tom & Jerry",
    "Скачивание датапака Dungeons & Taverns для Bob's World"
  ])
  expect(root.textContent).not.toContain("Server addon")
  expect(root.textContent).not.toMatch(/&(amp|#39|quot|lt|gt);/)

  const labels = [...root.querySelectorAll("section button")].map(
    (button) => button.textContent
  )
  expect(labels).toEqual([
    "Убрать все из списка",
    "Закрыть",
    "Убрать из списка",
    "Убрать из списка"
  ])
})

const open = async (tasks: unknown[]) => {
  c.tasks = tasks
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(() => <AddonFailures />, root)
  root.querySelector("button")!.click()
  await Promise.resolve()
  return root
}
const buttons = (root: HTMLElement) => [
  ...root.querySelectorAll<HTMLButtonElement>("section button")
]

it("F21 reads the task list when the core says it changed, not every second", async () => {
  await open([failed(1, "InstanceTaskInstallMod", { mod_name: "A" })])
  expect(c.options?.queryKey).toEqual(["vtask.getTasks"])
  expect(c.options?.refetchInterval).toBeUndefined()
})

it("F21 a list that could not refresh is read again when opened", async () => {
  c.isError = true
  await open([failed(1, "InstanceTaskInstallMod", { mod_name: "A" })])
  expect(c.refetch).toHaveBeenCalledTimes(1)
})

it("F21 Remove all removes every failure and carries on past one it cannot remove", async () => {
  c.query.mockImplementation(async () => ({ type: "Failed" }))
  c.mutation.mockImplementation(async ([, id]: [string, number]) => {
    if (id === 2) throw new Error("dismiss failed")
  })
  const root = await open([
    failed(1, "InstanceTaskInstallMod", { mod_name: "A" }),
    failed(2, "InstanceTaskInstallMod", { mod_name: "B" }),
    failed(3, "InstanceTaskInstallMod", { mod_name: "C" })
  ])
  const all = buttons(root)[0]
  expect(all.textContent).toBe("Убрать все из списка")
  all.click()
  await vi.waitFor(() => expect(c.refetch).toHaveBeenCalledTimes(1))
  expect(c.mutation.mock.calls.map(([call]) => call)).toEqual([
    ["vtask.dismissTask", 1],
    ["vtask.dismissTask", 2],
    ["vtask.dismissTask", 3]
  ])
  await vi.waitFor(() =>
    expect(root.querySelector('[role="alert"]')?.textContent).toBe(
      "Не удалось убрать из списка все ошибки. Попробуйте ещё раз."
    )
  )
  expect(buttons(root).every((button) => !button.disabled)).toBe(true)
})

it("F21 a single failure offers only its own Remove", async () => {
  const root = await open([
    failed(1, "InstanceTaskInstallMod", { mod_name: "A" })
  ])
  expect(buttons(root).map((button) => button.textContent)).toEqual([
    "Закрыть",
    "Убрать из списка"
  ])
})
