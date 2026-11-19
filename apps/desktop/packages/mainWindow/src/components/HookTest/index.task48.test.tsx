import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { createSignal } from "solid-js"
import { render } from "solid-js/web"
import type { FEHookTest } from "@gd/core_module/bindings"

const io = vi.hoisted(() => ({
  test: vi.fn(),
  stop: vi.fn()
}))
vi.mock("@gd/ui", () => ({
  toast: { error: vi.fn() },
  Spinner: () => <span data-testid="spinner" />,
  Button: (p: any) => (
    <button
      data-testid={p["data-testid"]}
      disabled={p.disabled}
      onClick={p.onClick}
    >
      {p.children}
    </button>
  )
}))
vi.mock("@gd/i18n", () => ({
  useTransContext: () => [
    (key: string, options?: Record<string, unknown>) =>
      options ? `${key} ${JSON.stringify(options)}` : key
  ]
}))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    createMutation: (options: () => { mutationKey: string[] }) =>
      options().mutationKey[0] === "instance.testHook"
        ? { mutateAsync: io.test }
        : { mutate: io.stop }
  }
}))

import HookTest, { seconds } from "./index"

let dispose: (() => void) | undefined
let root: HTMLElement
let setCommand: (value: string) => void
let setInstance: (value: number | undefined) => void
const flush = async () => {
  for (let i = 0; i < 8; i++) await Promise.resolve()
}
const part = (id: string) =>
  root.querySelector<HTMLElement>(`[data-testid="${id}"]`)
const result = (outcome: FEHookTest["outcome"], more = {}): FEHookTest => ({
  outcome,
  millis: 6234,
  stdout: "",
  stderr: "",
  ...more
})

beforeEach(() => {
  vi.clearAllMocks()
  vi.useFakeTimers()
  const [command, changeCommand] = createSignal("rclone sync saves remote:x")
  const [instance, changeInstance] = createSignal<number | undefined>(7)
  setCommand = changeCommand
  setInstance = changeInstance
  root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => (
      <HookTest instance={instance} kind="PostExit" command={command}>
        <input />
      </HookTest>
    ),
    root
  )
})
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
  vi.useRealTimers()
})

it("runs what the box holds now, shows that it runs, and then how it ended", async () => {
  let finish!: (value: FEHookTest) => void
  io.test.mockReturnValue(new Promise((resolve) => (finish = resolve)))
  setCommand("backup.sh --now")
  part("hook-test-run")!.click()
  await flush()
  expect(io.test).toHaveBeenCalledWith({
    instance: 7,
    kind: "PostExit",
    command: "backup.sh --now"
  })
  // While it runs: the time so far, and Stop in place of Test.
  expect(part("hook-test-run")).toBeNull()
  expect(part("hook-test-running")?.textContent).toContain('{"seconds":0}')
  vi.advanceTimersByTime(4000)
  expect(part("hook-test-running")?.textContent).toContain('{"seconds":4}')
  part("hook-test-stop")!.click()
  expect(io.stop).toHaveBeenCalledWith({ instance: 7, kind: "PostExit" })

  finish(
    result(
      { type: "Finished" },
      { stdout: "Transferred: 3 / 3\n", stderr: "note\n" }
    )
  )
  await flush()
  expect(part("hook-test-running")).toBeNull()
  expect(part("hook-test-verdict")?.textContent).toBe(
    'settings:_trn_hook_test_finished {"seconds":"6.2"}'
  )
  expect(part("hook-test-output")?.textContent).toBe(
    "Transferred: 3 / 3\nnote\n"
  )
  expect(part("hook-test-run")).not.toBeNull()
})

it("says each way a test did not work", async () => {
  const verdictOf = async (outcome: FEHookTest["outcome"], more = {}) => {
    io.test.mockResolvedValueOnce(result(outcome, more))
    part("hook-test-run")!.click()
    await flush()
    return part("hook-test-verdict")?.textContent
  }
  expect(
    await verdictOf(
      { type: "Failed", value: { code: 127 } },
      { stderr: "sh: rclone: command not found\n" }
    )
  ).toBe('settings:_trn_hook_test_failed_code {"code":127}')
  expect(part("hook-test-output")?.textContent).toBe(
    "sh: rclone: command not found\n"
  )
  expect(await verdictOf({ type: "Failed", value: { code: null } })).toBe(
    "settings:_trn_hook_test_failed"
  )
  // Nothing printed: no empty box.
  expect(part("hook-test-output")).toBeNull()
  expect(
    await verdictOf({
      type: "NotRun",
      value: { summary: "No such file or directory (os error 2)" }
    })
  ).toBe("No such file or directory (os error 2)")
  expect(await verdictOf({ type: "TimedOut" })).toBe(
    "settings:_trn_hook_test_timed_out"
  )
  expect(await verdictOf({ type: "Stopped" })).toBe(
    "settings:_trn_hook_test_stopped"
  )
  // The core refusing the test at all is shown in the same place.
  io.test.mockRejectedValueOnce(
    new Error("A test of this hook is already running.")
  )
  part("hook-test-run")!.click()
  await flush()
  expect(part("hook-test-verdict")?.textContent).toContain(
    "A test of this hook is already running."
  )
})

it("cannot test an empty command or without an instance to run in", () => {
  expect(part("hook-test-run")!.hasAttribute("disabled")).toBe(false)
  setCommand("   ")
  expect(part("hook-test-run")!.hasAttribute("disabled")).toBe(true)
  setCommand("backup.sh")
  setInstance(undefined)
  expect(part("hook-test-run")!.hasAttribute("disabled")).toBe(true)
  part("hook-test-run")!.click()
  expect(io.test).not.toHaveBeenCalled()
})

it("stops a test that is still running when its page goes away", async () => {
  io.test.mockReturnValue(new Promise(() => {}))
  part("hook-test-run")!.click()
  await flush()
  dispose?.()
  dispose = undefined
  expect(io.stop).toHaveBeenCalledWith({ instance: 7, kind: "PostExit" })
})

it("shows a test's time in seconds with one decimal", () => {
  expect(seconds(6234)).toBe("6.2")
  expect(seconds(40)).toBe("0.0")
})
