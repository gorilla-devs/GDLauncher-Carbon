import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { createSignal } from "solid-js"
import { render } from "solid-js/web"
import type { FEHookTest } from "@gd/core_module/bindings"

const io = vi.hoisted(() => ({
  test: vi.fn(),
  stop: vi.fn(),
  loading: vi.fn(),
  success: vi.fn(),
  error: vi.fn(),
  dismiss: vi.fn()
}))
vi.mock("@gd/ui", () => ({
  toast: {
    loading: io.loading,
    success: io.success,
    error: io.error,
    dismiss: io.dismiss
  },
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

import HookTest, { lastLines, seconds } from "./index"

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
/** The id of the notice the last test started. */
const notice = (): string => io.loading.mock.calls.at(-1)![1].id
/** What a notice says under its title. */
const told = (call: unknown[]) => {
  const options = call[1] as { description: HTMLElement }
  return {
    verdict: options.description.querySelector(
      '[data-testid="hook-test-verdict"]'
    )?.textContent,
    output:
      options.description.querySelector('[data-testid="hook-test-output"]')
        ?.textContent ?? null
  }
}

beforeEach(() => {
  vi.clearAllMocks()
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
})

it("runs what the box holds now and tells it in one notice, without moving the page", async () => {
  let finish!: (value: FEHookTest) => void
  io.test.mockReturnValue(new Promise((resolve) => (finish = resolve)))
  const before = root.innerHTML
  setCommand("backup.sh --now")
  part("hook-test-run")!.click()
  await flush()
  expect(io.test).toHaveBeenCalledWith({
    instance: 7,
    kind: "PostExit",
    command: "backup.sh --now"
  })
  // While it runs: a notice, and Stop in place of Test. Nothing else is
  // added to the page.
  expect(io.loading).toHaveBeenCalledWith(
    "settings:_trn_hook_test_title_post",
    {
      id: expect.stringMatching(/^hook-test-PostExit-\d+$/),
      duration: Infinity
    }
  )
  expect(part("hook-test-run")).toBeNull()
  expect(root.querySelectorAll("div, pre, span").length).toBe(
    new DOMParser()
      .parseFromString(before, "text/html")
      .querySelectorAll("div, pre, span").length
  )
  part("hook-test-stop")!.click()
  expect(io.stop).toHaveBeenCalledWith({ instance: 7, kind: "PostExit" })

  finish(
    result(
      { type: "Finished" },
      { stdout: "Transferred: 3 / 3\n", stderr: "note\n" }
    )
  )
  await flush()
  // The same notice turns into the outcome, and stays while there is
  // something to read.
  const [title, options] = io.success.mock.calls[0]
  expect(title).toBe("settings:_trn_hook_test_title_post")
  expect(options.id).toBe(notice())
  expect(options.duration).toBe(Infinity)
  expect(told(io.success.mock.calls[0])).toEqual({
    verdict: 'settings:_trn_hook_test_finished {"seconds":"6.2"}',
    output: "Transferred: 3 / 3\nnote"
  })
  expect(part("hook-test-run")).not.toBeNull()
  expect(root.innerHTML).toBe(before)
})

it("lets a test that printed nothing go by itself, and keeps every failure", async () => {
  io.test.mockResolvedValueOnce(result({ type: "Finished" }))
  part("hook-test-run")!.click()
  await flush()
  expect(io.success.mock.calls[0][1].duration).toBe(6000)
  expect(told(io.success.mock.calls[0]).output).toBeNull()

  const failure = async (outcome: FEHookTest["outcome"], more = {}) => {
    io.error.mockClear()
    io.test.mockResolvedValueOnce(result(outcome, more))
    part("hook-test-run")!.click()
    await flush()
    expect(io.error.mock.calls[0][1].duration).toBe(Infinity)
    expect(io.error.mock.calls[0][1].id).toBe(notice())
    return told(io.error.mock.calls[0])
  }
  expect(
    await failure(
      { type: "Failed", value: { code: 127 } },
      { stderr: "sh: rclone: command not found\n" }
    )
  ).toEqual({
    verdict: 'settings:_trn_hook_test_failed_code {"code":127}',
    output: "sh: rclone: command not found"
  })
  expect(
    (await failure({ type: "Failed", value: { code: null } })).verdict
  ).toBe("settings:_trn_hook_test_failed")
  expect(
    (
      await failure({
        type: "NotRun",
        value: { summary: "rclone was not found. Check that it is installed." }
      })
    ).verdict
  ).toBe("rclone was not found. Check that it is installed.")
  expect((await failure({ type: "TimedOut" })).verdict).toBe(
    "settings:_trn_hook_test_timed_out"
  )
  // The core refusing the test at all is told the same way.
  io.error.mockClear()
  io.test.mockRejectedValueOnce(
    new Error("A test of this hook is already running.")
  )
  part("hook-test-run")!.click()
  await flush()
  expect(told(io.error.mock.calls[0]).verdict).toContain(
    "A test of this hook is already running."
  )
})

it("gives each test a notice of its own and puts the last one away", async () => {
  // A notice reused would keep the last result under the spinner, and stay
  // behind any notice shown since.
  io.test.mockResolvedValueOnce(
    result({ type: "Failed", value: { code: 1 } }, { stderr: "no\n" })
  )
  part("hook-test-run")!.click()
  await flush()
  const first = notice()
  io.test.mockResolvedValueOnce(result({ type: "Finished" }))
  part("hook-test-run")!.click()
  await flush()
  expect(notice()).not.toBe(first)
  expect(io.dismiss).toHaveBeenCalledWith(first)
  expect(io.dismiss.mock.invocationCallOrder.at(-1)).toBeLessThan(
    io.loading.mock.invocationCallOrder.at(-1)!
  )
  expect(io.success.mock.calls[0][1].id).toBe(notice())
})

it("only takes the notice away for a test the player stopped", async () => {
  io.test.mockResolvedValueOnce(result({ type: "Stopped" }))
  part("hook-test-run")!.click()
  await flush()
  expect(io.dismiss).toHaveBeenLastCalledWith(notice())
  expect(io.success).not.toHaveBeenCalled()
  expect(io.error).not.toHaveBeenCalled()
  expect(part("hook-test-run")).not.toBeNull()
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

it("shows a test's time with one decimal, and only the end of a long output", () => {
  expect(seconds(6234)).toBe("6.2")
  expect(seconds(40)).toBe("0.0")
  expect(lastLines("one\ntwo\n")).toBe("one\ntwo")
  const long = Array.from({ length: 30 }, (_, n) => `line ${n}`).join("\n")
  expect(lastLines(`${long}\n`).split("\n")).toEqual(
    Array.from({ length: 8 }, (_, n) => `line ${n + 22}`)
  )
  expect(lastLines("")).toBe("")
})
