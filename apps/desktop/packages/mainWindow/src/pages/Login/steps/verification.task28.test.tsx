import { afterEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import { GdlAccountVerificationStep } from "./GdlAccountVerificationStep"
const c = vi.hoisted(() => ({
  fetch: vi.fn(),
  save: vi.fn(),
  resend: vi.fn(),
  exit: vi.fn(),
  success: vi.fn(),
  error: vi.fn()
}))
vi.mock("@/utils/rspcClient", () => ({
  apiFetch: c.fetch,
  rspc: {
    createMutation: (opts: any) => ({
      mutateAsync: opts().mutationKey[0].includes("save") ? c.save : c.resend
    })
  }
}))
vi.mock("../flow/FlowContext", () => ({
  useFlow: () => ({ exitFlow: c.exit, data: { isFirstLaunch: false } })
}))
vi.mock("@gd/i18n", () => ({ Trans: (p: any) => <span>{p.key}</span> }))
vi.mock("@gd/ui", () => ({
  Button: (p: any) => (
    <button disabled={p.disabled} onClick={p.onClick}>
      {p.children}
    </button>
  ),
  toast: { success: c.success, error: c.error }
}))
let dispose: (() => void) | undefined
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
  Object.values(c).forEach((v) => v.mockReset())
  vi.useRealTimers()
})
const flush = async () => {
  for (let i = 0; i < 10; i++) await Promise.resolve()
}
function fixture() {
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => (
      <GdlAccountVerificationStep
        step={{
          type: "gdl-account-verification",
          uuid: "synthetic-uuid",
          email: "synthetic@example.invalid"
        }}
      />
    ),
    root
  )
  return root
}
function pollQueue() {
  const requests: any[] = []
  c.fetch.mockImplementation(
    (_: string, { signal }: any) =>
      new Promise((resolve, reject) => {
        signal.addEventListener("abort", () =>
          reject(Object.assign(new Error("aborted"), { name: "AbortError" }))
        )
        requests.push({ signal, resolve, reject })
      })
  )
  return requests
}
it("GPT-0125 resend A's abort cannot erase B and cleanup revokes late B", async () => {
  const requests = pollQueue()
  c.resend.mockResolvedValue({ status: "success" })
  const root = fixture()
  await flush()
  root.querySelector("button")!.click()
  await flush()
  expect(requests).toHaveLength(2)
  expect(requests[0].signal.aborted).toBe(true)
  dispose!()
  dispose = undefined
  expect(requests[1].signal.aborted).toBe(true)
  requests[1].resolve({ ok: true, text: async () => "ok" })
  await flush()
  expect(c.save).not.toHaveBeenCalled()
  expect(c.exit).not.toHaveBeenCalled()
})
it("GPT-0125 cleanup while body is pending prevents save dispatch", async () => {
  const requests = pollQueue()
  let body!: (v: string) => void
  fixture()
  await flush()
  requests[0].resolve({ ok: true, text: () => new Promise((r) => (body = r)) })
  await flush()
  dispose!()
  dispose = undefined
  body("ok")
  await flush()
  expect(c.save).not.toHaveBeenCalled()
  expect(c.success).not.toHaveBeenCalled()
})
it("GPT-0125 late resend completion cannot restart after disposal", async () => {
  const requests = pollQueue()
  let finish!: (v: any) => void
  c.resend.mockImplementation(() => new Promise((r) => (finish = r)))
  const root = fixture()
  await flush()
  root.querySelector("button")!.click()
  dispose!()
  dispose = undefined
  finish({ status: "success" })
  await flush()
  expect(requests).toHaveLength(1)
  expect(c.success).not.toHaveBeenCalled()
})
it("live success saves and exits once; failed resend preserves the poll", async () => {
  const requests = pollQueue()
  c.resend.mockResolvedValue({ status: "failed", value: 2 })
  c.save.mockResolvedValue(undefined)
  const root = fixture()
  await flush()
  root.querySelector("button")!.click()
  await flush()
  expect(requests).toHaveLength(1)
  requests[0].resolve({ ok: true, text: async () => "ok" })
  await flush()
  expect(c.save).toHaveBeenCalledExactlyOnceWith("synthetic-uuid")
  expect(c.exit).toHaveBeenCalledExactlyOnceWith("library", false)
})
it("a live save already dispatched is retained, but disposal suppresses later toast/exit", async () => {
  const requests = pollQueue()
  let saved!: () => void
  c.save.mockImplementation(() => new Promise<void>((r) => (saved = r)))
  fixture()
  await flush()
  requests[0].resolve({ ok: true, text: async () => "ok" })
  await flush()
  expect(c.save).toHaveBeenCalledTimes(1)
  dispose!()
  dispose = undefined
  saved()
  await flush()
  expect(c.success).not.toHaveBeenCalled()
  expect(c.exit).not.toHaveBeenCalled()
})
it("no-resend cleanup aborts its poll and pending cooldown resend cannot publish", async () => {
  vi.useFakeTimers()
  const requests = pollQueue()
  let finish!: (v: any) => void
  c.resend.mockImplementation(() => new Promise((r) => (finish = r)))
  const root = fixture()
  await flush()
  root.querySelector("button")!.click()
  dispose!()
  dispose = undefined
  expect(requests[0].signal.aborted).toBe(true)
  finish({ status: "failed", value: 20 })
  await flush()
  expect(vi.getTimerCount()).toBe(0)
  expect(c.save).not.toHaveBeenCalled()
})
