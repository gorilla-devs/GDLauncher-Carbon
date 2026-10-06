import { afterEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import { createSignal } from "solid-js"
import EditShare from "./EditShare"
import ShareInstance from "./ShareInstance"
import * as validation from "@/utils/validation"

const io = vi.hoisted(() => ({
  update: vi.fn(),
  close: vi.fn(),
  success: vi.fn(),
  error: vi.fn(),
  streams: [] as any[]
}))
vi.mock("@/utils/rspcClient", () => ({
  apiUrl: (path: string) => `/inert${path}`,
  rspc: {
    createMutation: () => ({ mutateAsync: io.update, isPending: false }),
    createQuery: () => ({ data: undefined, error: undefined })
  }
}))
vi.mock("../index", () => ({ useModal: () => ({ closeModal: io.close }) }))
vi.mock("../ModalLayout", () => ({
  default: (p: any) => <div>{p.children}</div>
}))
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => <span>{p.key}</span>,
  useTransContext: () => [(key: string) => key]
}))
vi.mock("@solidjs/router", () => ({ useNavigate: () => () => {} }))
vi.mock("@/components/GlobalStoreContext", () => ({
  useGlobalStore: () => ({
    gdlAccount: { data: { status: "valid", value: { isEmailVerified: true } } }
  })
}))
vi.mock("@/components/VerificationRequiredPlaceholder", () => ({
  default: () => <div>verification required</div>
}))
vi.mock("@/components/SharePreviewContent", () => ({
  getErrorCode: (error: any) => error?.code
}))
vi.mock("@gd/ui", async () => ({
  ...(await import("../../../../../../../../packages/ui/src/Input")),
  ...(await import("../../../../../../../../packages/ui/src/Button")),
  ...(await import("../../../../../../../../packages/ui/src/Select")),
  ...(await import("../../../../../../../../packages/ui/src/Switch")),
  ...(await import("../../../../../../../../packages/ui/src/Progress")),
  Collapsable: (p: any) => <div>{p.children}</div>,
  CopyText: () => <div />,
  toast: { success: io.success, error: io.error }
}))

let dispose: (() => void) | undefined
afterEach(() => {
  dispose?.()
  dispose = undefined
  vi.unstubAllGlobals()
  vi.restoreAllMocks()
  io.update.mockReset()
  io.close.mockReset()
  io.success.mockReset()
  io.error.mockReset()
  io.streams = []
  document.body.textContent = ""
})
const flush = async () => {
  for (let i = 0; i < 15; i++) await Promise.resolve()
}
function mount(component: () => any) {
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(component, root)
  return root
}
function input(element: HTMLInputElement, text: string) {
  element.value = text
  element.dispatchEvent(new Event("input", { bubbles: true }))
}
function button(root: HTMLElement, suffix: string) {
  return [...root.querySelectorAll<HTMLButtonElement>("button")].find((b) =>
    b.textContent?.includes(suffix)
  )!
}
function edit(limit: any = 42, onUpdated = vi.fn()) {
  io.update.mockResolvedValue(undefined)
  const root = mount(() => (
    <EditShare
      title="Edit share"
      data={{
        share: { shareCode: "fixture", title: " Title ", maxDownloads: limit },
        onUpdated
      }}
    />
  ))
  return { root, onUpdated }
}
function share() {
  // This constructor records the produced URL; it never creates a stream/transport.
  vi.stubGlobal(
    "EventSource",
    class {
      onmessage: any
      closed = false
      constructor(readonly url: string) {
        io.streams.push(this)
      }
      addEventListener() {}
      close() {
        this.closed = true
      }
    }
  )
  return mount(() => (
    <ShareInstance title="Share instance" data={{ instanceId: 7 }} />
  ))
}

it.each([
  ["1e3", 1000, "1000"],
  ["42", 42, "42"],
  ["", null, ""],
  ["  ", null, ""],
  ["0", 1, "1"],
  ["-2", 1, "1"],
  ["10000001", 10000000, "10000000"],
  ["12chairs", null, ""],
  ["1.5", null, ""],
  ["Infinity", null, ""],
  ["NaN", null, ""],
  ["0x10", null, ""],
  ["0b10", null, ""],
  ["1e309", null, ""],
  ["12 3", null, ""],
  [" 4.2e1 ", 42, "42"],
  ["+42", 42, "42"]
] as const)(
  "GPT0260 complete decimal input %j has consistent numeric/display meaning",
  (text, value, display) => {
    expect(validation.maxDownloadsSchema.parse(text)).toBe(value)
    expect(validation.validateMaxDownloads(text)).toBe(display)
  }
)
it("nullable/unlimited schema and composed form preserve their contracts", () => {
  expect(validation.maxDownloadsSchema.parse(null)).toBeNull()
  expect(
    validation.shareInstanceFormSchema.parse({
      title: "title",
      expirationDays: "7",
      maxDownloads: "1e3"
    })
  ).toEqual({ title: "title", expirationDays: "7", maxDownloads: 1000 })
})
it.each([
  ["1e3", "1000", 1000],
  ["42", "42", 42],
  ["", "", null],
  ["0", "1", 1],
  ["10000001", "10000000", 10000000]
] as const)(
  "actual EditShare input %j displays %j and saves %s",
  async (text, display, value) => {
    const f = edit()
    const field = f.root.querySelector<HTMLInputElement>("input[type=number]")!
    input(field, text)
    expect(field.value).toBe(display)
    button(f.root, "_trn_my_shares.save").click()
    await flush()
    expect(io.update).toHaveBeenCalledWith({
      shareCode: "fixture",
      title: "Title",
      maxDownloads: value
    })
    expect(f.onUpdated).toHaveBeenCalledTimes(1)
    expect(io.close).toHaveBeenCalledTimes(1)
    expect(io.success).toHaveBeenCalledTimes(1)
  }
)
it.each([
  ["1e3", 1000],
  ["12chairs", null],
  ["10000001", 10000000]
] as const)(
  "EditShare validates a programmatic saved %j again at submission",
  async (raw, value) => {
    const f = edit(raw)
    button(f.root, "_trn_my_shares.save").click()
    await flush()
    expect(io.update.mock.calls[0][0].maxDownloads).toBe(value)
  }
)
it("EditShare failure retains callback/closure and shows only error feedback", async () => {
  const f = edit()
  io.update.mockRejectedValue(new Error("injected update failure"))
  button(f.root, "_trn_my_shares.save").click()
  await flush()
  expect(io.error).toHaveBeenCalledTimes(1)
  expect(io.success).not.toHaveBeenCalled()
  expect(f.onUpdated).not.toHaveBeenCalled()
  expect(io.close).not.toHaveBeenCalled()
})
it("EditShare with its share removed after mount dispatches nothing", async () => {
  const [data, setData] = createSignal<any>({
    share: { shareCode: "fixture", title: "title", maxDownloads: 42 },
    onUpdated: vi.fn()
  })
  const root = mount(() => <EditShare title="Edit share" data={data()} />)
  setData({ onUpdated: vi.fn() })
  button(root, "_trn_my_shares.save").click()
  await flush()
  expect(io.update).not.toHaveBeenCalled()
})
it.each([
  ["1e3", "1000"],
  ["42", "42"],
  ["", null],
  ["0", "1"],
  ["10000001", "10000000"]
] as const)(
  "actual ShareInstance input %j produces canonical URL limit %s",
  async (text, value) => {
    const root = share()
    input(root.querySelector<HTMLInputElement>("input[type=number]")!, text)
    input(
      root.querySelector<HTMLInputElement>("input:not([type=number])")!,
      " Title "
    )
    button(root, "_trn_instance_share.share_button").click()
    await flush()
    expect(io.streams).toHaveLength(1)
    const params = new URL(io.streams[0].url, "https://fixture.invalid")
      .searchParams
    expect(params.get("maxDownloads")).toBe(value)
    expect(params.get("title")).toBe("Title")
    expect(params.get("instanceId")).toBe("7")
    expect(params.get("expirationDays")).toBe("1")
    expect(params.has("includeSaves")).toBe(false)
    dispose!()
    dispose = undefined
    expect(io.streams[0].closed).toBe(true)
  }
)
it.each([
  ["1e3", "1000"],
  ["12chairs", null],
  ["10000001", "10000000"]
] as const)(
  "ShareInstance revalidates a stale programmatic draft %j at the URL boundary",
  async (raw, value) => {
    const root = share()
    // Simulate a stale draft admitted before normalization without changing the real schema/parser.
    vi.spyOn(validation, "validateMaxDownloads").mockReturnValueOnce(raw)
    input(root.querySelector<HTMLInputElement>("input[type=number]")!, "42")
    button(root, "_trn_instance_share.share_button").click()
    await flush()
    expect(
      new URL(io.streams[0].url, "https://fixture.invalid").searchParams.get(
        "maxDownloads"
      )
    ).toBe(value)
  }
)
it.each([
  [
    "NOT_ENOUGH_DISK_SPACE",
    "instances:_trn_share_errors.not_enough_disk_space"
  ],
  ["INSTANCE_TOO_LARGE", "instances:_trn_share_errors.instance_too_large"],
  ["UNKNOWN_ERROR", "instances:_trn_share_errors.upload_failed"]
] as const)(
  "a share that fails with %s tells the user %s",
  async (code, message) => {
    const root = share()
    button(root, "_trn_instance_share.share_button").click()
    await flush()
    io.streams[0].onmessage({
      data: JSON.stringify({ error: { code, message: "core message" } })
    })
    await flush()
    expect(io.error).toHaveBeenCalledWith(message)
    expect(root.textContent).toContain(message)
    expect(io.streams[0].closed).toBe(true)
  }
)
