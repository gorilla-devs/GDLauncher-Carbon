import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { createSignal } from "solid-js"
import { render } from "solid-js/web"
import RepairModpack from "."
import { button, deferred, settle } from "@/task33.fixture"
const c = vi.hoisted(() => ({
  preview: undefined as any,
  pending: undefined as any,
  details: undefined as any,
  outcome: undefined as any,
  mutate: vi.fn(),
  dismiss: vi.fn(),
  close: vi.fn(),
  navigate: vi.fn(),
  refetch: vi.fn(),
  queryKeys: [] as any[],
  resetOutcome: undefined as any,
  outcomeError: undefined as any,
  origin: vi.fn(),
  originRefetch: vi.fn(),
  previewRefetch: vi.fn(),
  pendingRefetch: vi.fn()
}))
vi.mock("@gd/ui", async () => import("@/task33.fixture"))
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => <span>{p.key}</span>,
  useTransContext: () => [(k: string) => k]
}))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ closeModal: c.close })
}))
vi.mock("@/managers/ModalsManager/ModalLayout", () => ({
  default: (p: any) => <div>{p.children}</div>
}))
vi.mock("@solidjs/router", () => ({
  useLocation: () => ({ pathname: "/library/7/settings" })
}))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ navigate: c.navigate })
}))
vi.mock("@/utils/rspcClient", async () => {
  const actual = await vi.importActual<any>("@/utils/rspcClient")
  return {
    ...actual,
    rspc: {
      useContext: () => ({
        client: { mutation: c.dismiss },
        queryClient: { invalidateQueries: c.refetch }
      }),
      createQuery: (f: any) => {
        c.queryKeys.push(f)
        const key = f().queryKey[0]
        if (key === "instance.getRepairPreview") return c.preview
        if (key === "instance.getPendingSetup") return c.pending
        if (key === "instance.getInstanceDetails") return c.details
        return {
          get data() {
            if (f().enabled === false) return undefined
            const current = c.outcome()
            return key === "vtask.getTask"
              ? current?.type === "Running" || current?.type === "Failed"
                ? current.value
                : current?.type === "Succeeded"
                  ? null
                  : undefined
              : current
          },
          get isError() {
            return c.outcomeError()
          },
          isLoading: false,
          refetch: c.originRefetch
        }
      },
      createMutation: (f: any) => ({
        isPending: false,
        mutateAsync: async (v: any) => {
          const o = f()
          c.mutate(o.mutationKey[0], v)
          c.resetOutcome()
          return c.origin(v)
        },
        mutate: (v: any) => {
          const o = f()
          c.mutate(o.mutationKey[0], v)
          if (o.mutationKey[0] === "instance.checkPackOrigin")
            o.onSuccess?.(
              c.mutate.mock.calls.filter((x) => x[0] === o.mutationKey[0])
                .length === 1
                ? 31
                : 32,
              v
            )
        }
      })
    }
  }
})
let dispose: undefined | (() => void),
  setPreview: any,
  setPending: any,
  setOutcome: any,
  setDetails: any,
  setOwner: any,
  setOutcomeError: any
const variant = { counts: { restore_modified: 1 }, entries: [] }
const file = {
  path: "mods/user.jar",
  size: 2,
  label: "Unknown",
  deletable: true,
  origin: null,
  selection: null
}
const ready = () => ({
  data: {
    has_packinfo: true,
    with_re_enable: variant,
    without_re_enable: variant,
    untracked: [file],
    duplicates: []
  },
  isSuccess: true,
  isLoading: false,
  isFetching: false,
  isError: false,
  isPlaceholderData: false
})
function reactive(initial: any, refetch = c.refetch) {
  const [s, set] = createSignal(initial)
  return [
    new Proxy(
      {},
      { get: (_, key) => (key === "refetch" ? refetch : s()[key]) }
    ),
    set
  ] as const
}
beforeEach(() => {
  vi.clearAllMocks()
  c.dismiss.mockReset().mockResolvedValue(undefined)
  c.queryKeys = []
  c.origin.mockReset().mockResolvedValueOnce(31).mockResolvedValue(32)
  c.originRefetch.mockReset().mockResolvedValue(undefined)
  c.previewRefetch.mockReset().mockImplementation(() => c.refetch())
  c.pendingRefetch.mockReset().mockImplementation(() => c.refetch())
  ;[c.preview, setPreview] = reactive(ready(), c.previewRefetch)
  ;[c.pending, setPending] = reactive(
    {
      data: { pending: false },
      isSuccess: true,
      isFetching: false,
      isError: false
    },
    c.pendingRefetch
  )
  ;[c.details, setDetails] = reactive({
    data: { modpack: { modpack: { type: "modrinth" } } }
  })
  const [s, set] = createSignal<any>()
  c.outcome = s
  setOutcome = set
  c.resetOutcome = () => set(undefined)
  const [error, setError] = createSignal(false)
  c.outcomeError = error
  setOutcomeError = setError
})
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
})
function mount(isServer = false) {
  const [owner, set] = createSignal<any>({ id: 7, name: "Synthetic", isServer })
  setOwner = set
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => <RepairModpack title="Synthetic" data={owner()} />,
    root
  )
  return root
}
const start = (root: HTMLElement) =>
  root.querySelector('[data-testid="repair-check-origin"]') as HTMLButtonElement
const confirm = (root: HTMLElement) =>
  root.querySelector(
    '[data-testid="repair-modpack-confirm"]'
  ) as HTMLButtonElement
const failure = {
  type: "Failed",
  value: {
    progress: {
      type: "Failed",
      value: { cause: [{ display: "Versions lookup failed" }] }
    },
    active_subtasks: []
  }
}
it("0207 owned terminal origin failure releases loading and shows actionable retry", async () => {
  const root = mount()
  start(root).click()
  await settle()
  expect(start(root).disabled).toBe(true)
  setOutcome(failure)
  await settle()
  expect(start(root).disabled).toBe(false)
  expect(root.textContent).toContain("Versions lookup failed")
  start(root).click()
  await settle()
  expect(c.dismiss).toHaveBeenCalledExactlyOnceWith(["vtask.dismissTask", 31])
  expect(
    c.mutate.mock.calls.filter((x) => x[0] === "instance.checkPackOrigin")
  ).toHaveLength(2)
})
it("0207 exact dismissal rejection is handled, retains error, and no retry work starts", async () => {
  c.dismiss.mockRejectedValue(new Error("ack denied"))
  const root = mount()
  start(root).click()
  await settle()
  setOutcome(failure)
  await settle()
  expect(start(root).disabled).toBe(false)
  start(root).click()
  await settle()
  expect(root.textContent).toContain("ack denied")
  expect(
    c.mutate.mock.calls.filter((x) => x[0] === "instance.checkPackOrigin")
  ).toHaveLength(1)
})
it("0207 pending/read absence and Known progress keep accepted origin attempt busy", async () => {
  const root = mount()
  start(root).click()
  await settle()
  setOutcome(undefined)
  await settle()
  expect(start(root).disabled).toBe(true)
  setOutcome({
    type: "Running",
    value: { progress: { type: "Known", value: 0.4 } }
  })
  expect(start(root).disabled).toBe(true)
  expect(confirm(root).disabled).toBe(false)
})
it("0207 succeeded outcome refreshes current preview and enables next check", async () => {
  const root = mount()
  start(root).click()
  await settle()
  setOutcome({ type: "Succeeded" })
  await settle()
  expect(start(root).disabled).toBe(false)
  expect(root.textContent).not.toContain("Versions lookup failed")
  expect(c.refetch).toHaveBeenCalled()
})
it("0207 disposed late dismissal cannot start new check", async () => {
  const p = deferred()
  c.dismiss.mockReturnValue(p.promise)
  const root = mount()
  start(root).click()
  await settle()
  setOutcome(failure)
  await settle()
  expect(start(root).disabled).toBe(false)
  start(root).click()
  dispose?.()
  dispose = undefined
  p.resolve(undefined)
  await settle()
  expect(
    c.mutate.mock.calls.filter((x) => x[0] === "instance.checkPackOrigin")
  ).toHaveLength(1)
})
it("0244 existing Task8 native error guard stays disabled; inline translated error and Retry become usable", async () => {
  setPreview({ ...ready(), data: undefined, isSuccess: false, isError: true })
  const root = mount()
  expect(confirm(root).disabled).toBe(true)
  confirm(root).click()
  expect(c.mutate).not.toHaveBeenCalled()
  expect(root.textContent).toContain("repair_preview_failed")
  button(root, "retry").click()
  await settle()
  expect(c.refetch).toHaveBeenCalled()
})
it("0244 forced handler on failed preview cannot close navigate or mutate", () => {
  setPreview({ ...ready(), data: undefined, isSuccess: false, isError: true })
  const root = mount()
  void (confirm(root) as any).invoke()
  expect(c.close).not.toHaveBeenCalled()
  expect(c.navigate).not.toHaveBeenCalled()
  expect(c.mutate).not.toHaveBeenCalled()
})
it.each(["isFetching", "isPlaceholderData"])(
  "0244 uncertain %s preview cannot authorize repair",
  (state) => {
    setPreview({ ...ready(), [state]: true })
    const root = mount()
    expect(confirm(root).disabled).toBe(true)
    void (confirm(root) as any).invoke()
    expect(c.mutate).not.toHaveBeenCalled()
  }
)
it("0244 pending-status refetch cannot authorize cached ready preview", () => {
  setPending({
    data: { pending: false },
    isSuccess: true,
    isFetching: true,
    isError: false
  })
  const root = mount()
  expect(confirm(root).disabled).toBe(true)
  void (confirm(root) as any).invoke()
  expect(c.mutate).not.toHaveBeenCalled()
})
it("0244 has_packinfo false remains supported current success and server remains preview-free", () => {
  const state = ready()
  state.data.has_packinfo = false
  setPreview(state)
  const root = mount()
  expect(root.textContent).toContain("repair_no_record")
  expect(confirm(root).disabled).toBe(false)
  confirm(root).click()
  expect(c.mutate).toHaveBeenCalledWith("instance.repairModpack", {
    instance: 7,
    cleanup_paths: [],
    cleanup_selections: [],
    re_enable_disabled: false
  })
  expect(c.close).toHaveBeenCalledTimes(1)
  dispose?.()
  dispose = undefined
  setPreview({ ...ready(), isError: true, isSuccess: false })
  setPending({ isSuccess: false, isError: true })
  const other = mount(true)
  expect(confirm(other).disabled).toBe(false)
  confirm(other).click()
  expect(c.mutate).toHaveBeenCalledWith("server.reinstallServer", 7)
})
it("0207 CurseForge availability guard stays hidden", () => {
  setDetails({ data: { modpack: { modpack: { type: "curseforge" } } } })
  const root = mount()
  expect(start(root)).toBeNull()
  expect(confirm(root).disabled).toBe(false)
})

it("0207 accepted transport read failure retains work and Retry only refreshes its query", async () => {
  const root = mount()
  start(root).click()
  await settle()
  setOutcomeError(true)
  expect(start(root).disabled).toBe(true)
  expect(root.textContent).toContain("repair_origin_read_failed")
  root
    .querySelector<HTMLButtonElement>('[data-testid="repair-origin-refresh"]')!
    .click()
  await settle()
  expect(c.originRefetch).toHaveBeenCalledTimes(1)
  expect(c.origin).toHaveBeenCalledTimes(1)
  setOutcomeError(false)
  setOutcome({ type: "Succeeded" })
  await settle()
  expect(start(root).disabled).toBe(false)
})
it("0207 Unavailable cannot retire accepted work or dispatch a duplicate origin check", async () => {
  const root = mount()
  start(root).click()
  await settle()
  setOutcome({ type: "Unavailable" })
  expect(start(root).disabled).toBe(true)
  void (start(root) as any).invoke()
  expect(c.origin).toHaveBeenCalledTimes(1)
  expect(root.textContent).toContain("repair_origin_unavailable")
})
it("0207 blank owned failure uses nonempty translated fallback", async () => {
  const root = mount()
  start(root).click()
  await settle()
  setOutcome({
    type: "Failed",
    value: {
      progress: { type: "Failed", value: { cause: [{ display: "  " }] } }
    }
  })
  expect(root.textContent).toContain("repair_origin_failed")
  expect(start(root).disabled).toBe(false)
})
it("0207 pending dismissal direct repeat is inert and replacement ABA revokes continuation", async () => {
  const ack = deferred()
  c.dismiss.mockReturnValue(ack.promise)
  const root = mount()
  start(root).click()
  await settle()
  setOutcome(failure)
  start(root).click()
  void (start(root) as any).invoke()
  await settle()
  expect(c.dismiss).toHaveBeenCalledTimes(1)
  setOwner({ id: 8, name: "B", isServer: false })
  setOwner({ id: 7, name: "A", isServer: false })
  ack.resolve(undefined)
  await settle()
  expect(c.origin).toHaveBeenCalledTimes(1)
  expect(start(root).disabled).toBe(false)
  expect(root.textContent).not.toContain("Versions lookup failed")
})
it("0207 request rejection releases loading with readable serialized feedback", async () => {
  c.origin.mockReset().mockRejectedValue(
    new Error(
      JSON.stringify({
        code: 500,
        message: JSON.stringify({
          cause: [{ display: "Readable origin request failure" }],
          backtrace: "synthetic secret-free trace"
        })
      })
    )
  )
  const root = mount()
  start(root).click()
  await settle()
  expect(start(root).disabled).toBe(false)
  expect(root.textContent).toContain("Readable origin request failure")
  expect(root.textContent).not.toContain('"code"')
  expect(root.textContent).not.toContain("synthetic secret-free trace")
})
it("0244 pending status error offers Retry without bypassing current admission", async () => {
  setPending({
    data: undefined,
    isSuccess: false,
    isError: true,
    isFetching: false
  })
  const root = mount()
  expect(confirm(root).disabled).toBe(true)
  button(root, "retry").click()
  await settle()
  expect(c.pendingRefetch).toHaveBeenCalledTimes(1)
  expect(c.previewRefetch).not.toHaveBeenCalled()
  expect(c.mutate).not.toHaveBeenCalled()
})
it("0244 delayed Retry cannot refetch a replaced instance or unresolved current setup", async () => {
  setPreview({ ...ready(), isError: true, isSuccess: false })
  const wait = deferred()
  c.pendingRefetch.mockReturnValue(wait.promise)
  const root = mount()
  button(root, "retry").click()
  setOwner({ id: 8, name: "B", isServer: false })
  wait.resolve(undefined)
  await settle()
  expect(c.previewRefetch).not.toHaveBeenCalled()
  expect(c.mutate).not.toHaveBeenCalled()
})
it("0244 normal cleanup and re-enable preserve Task3 frozen displayed selections", () => {
  const selection = {
    path: file.path,
    sha512: "a".repeat(128),
    proven: false,
    context: { platform: "modrinth", project_id: "pack-A", version_id: "v1" }
  }
  const state = ready()
  state.data.untracked = [{ ...file, selection } as any]
  setPreview(state)
  const root = mount()
  root
    .querySelector<HTMLElement>('[data-testid="repair-untracked-checkbox"]')!
    .querySelector<HTMLInputElement>("input")!
    .click()
  root
    .querySelector<HTMLElement>('[data-testid="repair-reenable-checkbox"]')!
    .closest("label")!
    .querySelector<HTMLInputElement>("input")!
    .click()
  confirm(root).click()
  expect(c.mutate).toHaveBeenCalledWith("instance.repairModpack", {
    instance: 7,
    cleanup_paths: [],
    cleanup_selections: [selection],
    re_enable_disabled: true
  })
})
it("0244 changed cleanup content/context reconciles prior selection before Confirm", () => {
  const selection = {
    path: file.path,
    sha512: "a".repeat(128),
    proven: false,
    context: { platform: "modrinth", project_id: "pack-A", version_id: "v1" }
  }
  const state = ready()
  state.data.untracked = [{ ...file, selection } as any]
  setPreview(state)
  const root = mount()
  root
    .querySelector<HTMLElement>('[data-testid="repair-untracked-checkbox"]')!
    .querySelector<HTMLInputElement>("input")!
    .click()
  const replacement = ready()
  replacement.data.untracked = [
    {
      ...file,
      selection: {
        ...selection,
        context: { ...selection.context, version_id: "v2" }
      }
    } as any
  ]
  setPreview(replacement)
  confirm(root).click()
  expect(c.mutate).toHaveBeenCalledWith("instance.repairModpack", {
    instance: 7,
    cleanup_paths: [],
    cleanup_selections: [],
    re_enable_disabled: false
  })
})
