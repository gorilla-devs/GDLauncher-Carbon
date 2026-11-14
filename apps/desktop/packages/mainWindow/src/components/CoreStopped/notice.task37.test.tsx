import { afterEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"

vi.mock("@gd/ui", () => ({
  Button: (props: any) => (
    <button
      data-testid={props["data-testid"]}
      disabled={props.disabled}
      onClick={props.onClick}
    >
      {props.children}
    </button>
  )
}))
vi.mock("@gd/i18n", () => ({
  Trans: (props: any) => (
    <span>
      {props.key}
      {props.options ? ` ${JSON.stringify(props.options)}` : ""}
    </span>
  )
}))
vi.mock("@/utils/adhelper", async () => {
  const { createSignal } = await import("solid-js")
  const { createStore } = await import("solid-js/store")
  const [adSize] = createStore({ width: 400, height: 600, shouldShow: true })
  const [adLayoutMounted, setAdLayoutMounted] = createSignal(false)
  return { default: adSize, adLayoutMounted, setAdLayoutMounted }
})

const { default: CoreStopped } = await import(".")
const { coreExitCode, coreStopped, resetCoreStopped } =
  await import("@/utils/coreStopped")
const { setAdLayoutMounted } = (await import("@/utils/adhelper")) as any

interface Exit {
  code: number | null
  signal: string | null
}
let dispose = () => {}
afterEach(() => {
  dispose()
  dispose = () => {}
  document.body.innerHTML = ""
  resetCoreStopped()
  setAdLayoutMounted(false)
  vi.restoreAllMocks()
})
function mount(already: Exit | null = null) {
  let deliver!: (exit: Exit) => void
  const stop = vi.fn()
  const relaunch = vi.fn(async (): Promise<unknown> => ({ status: "kept" }))
  const closeWindow = vi.fn(async () => {})
  Object.assign(window, {
    onCoreExited: (cb: typeof deliver) => {
      deliver = cb
      return stop
    },
    coreExit: async () => already,
    relaunch,
    closeWindow
  })
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(() => <CoreStopped />, root)
  return { deliver: (exit: Exit) => deliver(exit), stop, relaunch, closeWindow }
}
const notice = () =>
  document.querySelector<HTMLElement>('[data-testid="core-stopped"]')
const part = (id: string) =>
  document.querySelector<HTMLButtonElement>(`[data-testid="${id}"]`)!
const settled = async () => {
  for (let i = 0; i < 10; i++) await Promise.resolve()
}

it("shows nothing while the background service runs", async () => {
  mount()
  await settled()
  expect(notice()).toBeNull()
  expect(coreStopped()).toBeUndefined()
})

it("says the background service stopped as soon as main reports it", async () => {
  const f = mount()
  await settled()
  f.deliver({ code: 101, signal: null })
  const shown = notice()!
  expect(shown.textContent).toContain("app:_trn_core_stopped.title")
  expect(shown.textContent).toContain("app:_trn_core_stopped.body")
  expect(part("core-stopped-detail").textContent).toBe(
    'app:_trn_core_stopped.exit_code {"code":"101"}'
  )
  expect(part("core-stopped-restart").textContent).toBe(
    "settings:_trn_restart_app"
  )
  expect(part("core-stopped-quit").textContent).toBe(
    "window:_trn_window_close_quit_app"
  )
})

it("a window loaded after the stop shows it too", async () => {
  mount({ code: null, signal: "SIGKILL" })
  await settled()
  expect(notice()).not.toBeNull()
  // A signal is its own name: it is not an exit code.
  expect(part("core-stopped-detail").textContent).toBe("SIGKILL")
})

it("names how it ended for support, and leaves the line out when unknown", async () => {
  expect(coreExitCode({ code: 1, signal: null })).toBe("1")
  // Windows reports a crash as an NTSTATUS, which is looked up in hex.
  expect(coreExitCode({ code: 3221225477, signal: null })).toBe("0xC0000005")
  expect(coreExitCode({ code: null, signal: "SIGSEGV" })).toBeUndefined()
  const f = mount()
  f.deliver({ code: null, signal: null })
  expect(notice()).not.toBeNull()
  expect(document.querySelector('[data-testid="core-stopped-detail"]')).toBe(
    null
  )
})

it("Restart restarts GDLauncher once, and can be used again if that did not happen", async () => {
  const f = mount()
  f.deliver({ code: 101, signal: null })
  let finish!: (value: unknown) => void
  f.relaunch.mockImplementationOnce(
    () => new Promise((resolve) => (finish = resolve))
  )
  part("core-stopped-restart").click()
  expect(f.relaunch).toHaveBeenCalledTimes(1)
  expect(part("core-stopped-restart").disabled).toBe(true)
  finish({ status: "kept" })
  await settled()
  expect(part("core-stopped-restart").disabled).toBe(false)

  const error = vi.spyOn(console, "error").mockImplementation(() => {})
  f.relaunch.mockRejectedValueOnce(new Error("already closing"))
  part("core-stopped-restart").click()
  await settled()
  expect(error).toHaveBeenCalled()
  expect(part("core-stopped-restart").disabled).toBe(false)
  expect(notice()).not.toBeNull()
})

it("Quit App closes GDLauncher", () => {
  const f = mount()
  f.deliver({ code: 101, signal: null })
  part("core-stopped-quit").click()
  expect(f.closeWindow).toHaveBeenCalledTimes(1)
  expect(f.relaunch).not.toHaveBeenCalled()
})

it("leaves the ad column uncovered by its content, as dialogs do", () => {
  const f = mount()
  f.deliver({ code: 101, signal: null })
  expect(notice()!.style.paddingRight).toBe("")
  setAdLayoutMounted(true)
  expect(notice()!.style.paddingRight).toBe("432px")
})

it("stops listening when it is removed", () => {
  const f = mount()
  dispose()
  dispose = () => {}
  expect(f.stop).toHaveBeenCalledTimes(1)
})
