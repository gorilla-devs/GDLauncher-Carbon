import { afterEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import WindowCloseWarning from "./WindowCloseWarning"
import {
  confirmShutdownStops,
  listenShutdownQuestion,
  type ShutdownQuestion
} from "@/utils/shutdownQuestion"

const io = vi.hoisted(() => ({
  close: vi.fn(),
  mutate: vi.fn(),
  settings: { data: { showAppCloseWarning: true } as any }
}))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    createMutation: () => ({ mutate: io.mutate }),
    createQuery: () => io.settings
  }
}))
vi.mock("../index", () => ({ useModal: () => ({ closeModal: io.close }) }))
vi.mock("../ModalLayout", () => ({
  default: (p: any) => (
    <div data-testid="layout" data-title={p.title} data-height={p.height ?? ""}>
      {p.children}
    </div>
  )
}))
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => <span>{p.key}</span>,
  useTransContext: () => [(key: string) => key]
}))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ navigate: () => {} })
}))
vi.mock("@gd/ui", async () => ({
  ...(await import("../../../../../../../../packages/ui/src/Button")),
  ...(await import("../../../../../../../../packages/ui/src/Checkbox"))
}))

let dispose: (() => void) | undefined
afterEach(() => {
  dispose?.()
  dispose = undefined
  vi.unstubAllGlobals()
  io.close.mockReset()
  io.mutate.mockReset()
  document.body.textContent = ""
})
function mount(data?: Partial<ShutdownQuestion>) {
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => <WindowCloseWarning title="modals:_trn_confirm_quit" data={data} />,
    root
  )
  return root
}
const testIds = (root: HTMLElement) =>
  [...root.querySelectorAll<HTMLElement>("[data-testid]")]
    .map((element) => element.dataset.testid)
    .filter((id) => id !== "layout")
const click = (root: HTMLElement, testId: string) =>
  root.querySelector<HTMLElement>(`[data-testid="${testId}"]`)!.click()
const layout = (root: HTMLElement) =>
  root.querySelector<HTMLElement>('[data-testid="layout"]')!

it("a game alone gets the game warning as it was", () => {
  const closeWindow = vi.fn()
  vi.stubGlobal("closeWindow", closeWindow)
  for (const data of [
    undefined,
    { action: "quit" as const, game: true, servers: false, installs: false }
  ]) {
    const root = mount(data)
    expect(root.textContent).toContain("window:_trn_window_close_title")
    expect(root.textContent).toContain("window:_trn_window_close_text_1")
    expect(testIds(root)).toEqual(["window-close-warning-quit"])
    expect(layout(root).dataset.title).toBe("modals:_trn_confirm_quit")
    expect(layout(root).dataset.height).toBe("h-70")
    click(root, "window-close-warning-quit")
    dispose?.()
    dispose = undefined
  }
  expect(closeWindow).toHaveBeenCalledTimes(2)
})

it("a quit says what it stops, one line each, and that a game keeps running", () => {
  const root = mount({
    action: "quit",
    game: true,
    servers: true,
    installs: true
  })
  expect(testIds(root)).toEqual([
    "window-close-stops",
    "close-stops-servers",
    "close-stops-installs",
    "close-game-keeps-running",
    "window-close-warning-cancel",
    "window-close-warning-quit"
  ])
  expect(root.textContent).not.toContain("window:_trn_window_close_title")
  expect(layout(root).dataset.title).toBe("modals:_trn_confirm_quit")
  // Sized by its lines.
  expect(layout(root).dataset.height).toBe("")
})

it("lists only what is running", () => {
  const servers = mount({
    action: "quit",
    game: false,
    servers: true,
    installs: false
  })
  expect(testIds(servers)).toEqual([
    "window-close-stops",
    "close-stops-servers",
    "window-close-warning-cancel",
    "window-close-warning-quit"
  ])
  dispose?.()
  const installs = mount({
    action: "quit",
    game: false,
    servers: false,
    installs: true
  })
  expect(testIds(installs)).toEqual([
    "window-close-stops",
    "close-stops-installs",
    "window-close-warning-cancel",
    "window-close-warning-quit"
  ])
})

it("Quit App closes the window and Cancel only closes the question", () => {
  const closeWindow = vi.fn()
  vi.stubGlobal("closeWindow", closeWindow)
  const root = mount({
    action: "quit",
    game: false,
    servers: true,
    installs: false
  })
  click(root, "window-close-warning-cancel")
  expect(io.close).toHaveBeenCalledTimes(1)
  expect(closeWindow).not.toHaveBeenCalled()
  click(root, "window-close-warning-quit")
  expect(closeWindow).toHaveBeenCalledTimes(1)
})

it("a restart and an update have their own title and button and answer yes only from that button", () => {
  for (const [action, title, label] of [
    [
      "restart",
      "window:_trn_window_close_restart_title",
      "settings:_trn_restart_app"
    ],
    [
      "update",
      "window:_trn_window_close_update_title",
      "app:_trn_update_install_button"
    ]
  ] as const) {
    const answer = vi.fn()
    const root = mount({
      action,
      // A restart leaves a running game alone, so it is not mentioned.
      game: true,
      servers: true,
      installs: true,
      answer
    })
    expect(layout(root).dataset.title).toBe(title)
    expect(testIds(root)).toEqual([
      "window-close-stops",
      "close-stops-servers",
      "close-stops-installs",
      "window-close-warning-cancel",
      "window-close-warning-confirm"
    ])
    const confirm = root.querySelector<HTMLElement>(
      '[data-testid="window-close-warning-confirm"]'
    )!
    expect(confirm.textContent).toBe(label)
    confirm.click()
    expect(answer.mock.calls).toEqual([[true]])
    expect(io.close).toHaveBeenCalledTimes(1)
    // Closing afterwards does not answer a second time.
    dispose?.()
    dispose = undefined
    expect(answer.mock.calls).toEqual([[true]])
    io.close.mockReset()
  }
})

it("closing a restart question any other way answers no", () => {
  const answer = vi.fn()
  const root = mount({
    action: "restart",
    game: false,
    servers: true,
    installs: false,
    answer
  })
  click(root, "window-close-warning-cancel")
  expect(io.close).toHaveBeenCalledTimes(1)
  // The modal manager removes it: its X, Escape, the backdrop or Cancel.
  dispose?.()
  dispose = undefined
  expect(answer.mock.calls).toEqual([[false]])
})

it("Never show this again switches the close warning's setting", () => {
  const root = mount({
    action: "quit",
    game: false,
    servers: true,
    installs: false
  })
  root.querySelector<HTMLInputElement>('input[type="checkbox"]')!.click()
  expect(io.mutate).toHaveBeenCalledWith({
    showAppCloseWarning: { Set: false }
  })
})

it("a restart goes ahead without a question when nothing would be stopped or main cannot say", async () => {
  const asked = vi.fn()
  const stop = listenShutdownQuestion(asked)
  vi.stubGlobal("shutdownStops", async () => ({
    servers: false,
    installs: false
  }))
  expect(await confirmShutdownStops("restart")).toBe(true)
  vi.stubGlobal("shutdownStops", async () => {
    throw new Error("main did not answer")
  })
  const warn = vi.spyOn(console, "warn").mockImplementation(() => {})
  expect(await confirmShutdownStops("update")).toBe(true)
  warn.mockRestore()
  vi.stubGlobal("shutdownStops", undefined)
  expect(await confirmShutdownStops("restart")).toBe(true)
  expect(asked).not.toHaveBeenCalled()
  stop()
})

it("a restart waits for the answer to its question", async () => {
  vi.stubGlobal("shutdownStops", async () => ({
    servers: true,
    installs: false
  }))
  const questions: ShutdownQuestion[] = []
  const stop = listenShutdownQuestion((question) => questions.push(question))
  let settled: boolean | undefined
  const pending = confirmShutdownStops("restart").then((value) => {
    settled = value
  })
  for (let i = 0; i < 10; i++) await Promise.resolve()
  expect(questions).toHaveLength(1)
  expect(questions[0]).toMatchObject({
    action: "restart",
    game: false,
    servers: true,
    installs: false
  })
  expect(settled).toBeUndefined()
  questions[0].answer!(false)
  await pending
  expect(settled).toBe(false)

  const again = confirmShutdownStops("update")
  for (let i = 0; i < 10; i++) await Promise.resolve()
  expect(questions[1].action).toBe("update")
  questions[1].answer!(true)
  expect(await again).toBe(true)
  stop()
})

it("with nothing to show the question, the restart is not left waiting", async () => {
  vi.stubGlobal("shutdownStops", async () => ({
    servers: true,
    installs: true
  }))
  expect(await confirmShutdownStops("restart")).toBe(true)
})
