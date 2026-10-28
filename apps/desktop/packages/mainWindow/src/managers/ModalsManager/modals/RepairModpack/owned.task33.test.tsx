import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { createSignal } from "solid-js"
import { render } from "solid-js/web"
import RepairModpack from "."
import { button, deferred, settle, toast } from "@/task33.fixture"
const c = vi.hoisted(() => ({
  preview: undefined as any,
  pending: undefined as any,
  details: undefined as any,
  modpackInfo: undefined as any,
  mrVersion: undefined as any,
  cfFile: undefined as any,
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
  pendingRefetch: vi.fn(),
  // The error each mutation key is refused with, and the options each was
  // last called with.
  failing: {} as Record<string, Error>,
  options: {} as Record<string, any>
}))
vi.mock("@gd/ui", async () => import("@/task33.fixture"))
// Escapes interpolated values the way i18next does by default, unless the
// call opts out.
const escapeLikeI18next = (value: string) =>
  value.replace(/&/g, "&amp;").replace(/'/g, "&#39;").replace(/\//g, "&#x2F;")
vi.mock("@gd/i18n", () => ({
  i18n: { t: (k: string) => k },
  Trans: (p: any) => <span>{p.key}</span>,
  useTransContext: () => [
    (
      k: string,
      o?: { version?: string; interpolation?: { escapeValue?: boolean } }
    ) =>
      o?.version === undefined
        ? k
        : `${k}(${
            o.interpolation?.escapeValue === false
              ? o.version
              : escapeLikeI18next(o.version)
          })`
  ]
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
        if (key === "instance.getModpackInfo") return c.modpackInfo
        if (key === "modplatforms.modrinth.getVersion")
          return {
            get data() {
              return f().enabled ? c.mrVersion(f().queryKey[1]) : undefined
            }
          }
        if (key === "modplatforms.curseforge.getModFile")
          return {
            get data() {
              return f().enabled ? c.cfFile(f().queryKey[1]) : undefined
            }
          }
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
          c.options[o.mutationKey[0]] = o
          const error = c.failing[o.mutationKey[0]]
          if (error) return o.onError?.(error, v)
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
  c.failing = {}
  c.options = {}
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
    data: {
      modpack: {
        modpack: {
          type: "modrinth",
          value: { project_id: "pack", version_id: "v1" }
        }
      }
    }
  })
  c.modpackInfo = { data: undefined }
  c.mrVersion = () => undefined
  c.cfFile = () => undefined
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
  root.querySelector<HTMLButtonElement>('[data-testid="repair-check-origin"]')!
const confirm = (root: HTMLElement) =>
  root.querySelector<HTMLButtonElement>(
    '[data-testid="repair-modpack-confirm"]'
  )!
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
it("background pending-status refetch keeps Confirm enabled on the last status", () => {
  setPending({
    data: { pending: false },
    isSuccess: true,
    isFetching: true,
    isError: false
  })
  const root = mount()
  expect(confirm(root).disabled).toBe(false)
  confirm(root).click()
  // The core refuses it if a setup became pending since that status.
  expect(c.mutate).toHaveBeenCalledWith("instance.repairModpack", {
    instance: 7,
    cleanup_paths: [],
    cleanup_selections: [],
    re_enable_disabled: false,
    expected_pending: null
  })
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
    re_enable_disabled: false,
    expected_pending: null
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
    re_enable_disabled: true,
    expected_pending: null
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
    re_enable_disabled: false,
    expected_pending: null
  })
})

const pendingStatus = (status: Record<string, unknown>) => ({
  data: {
    pending: true,
    operation: "op-1",
    installed_version: "v1",
    pending_version: "v2",
    can_cancel: false,
    reason: "A reason the dialog no longer shows",
    repair_version: "v1",
    preview_withheld: false,
    can_resume: true,
    busy: false,
    discard_token: "op-1",
    ...status
  },
  isSuccess: true,
  isFetching: false,
  isError: false
})
/** Unmounts the dialog so the next `mount()` starts from fresh state. */
function unmount() {
  dispose?.()
  dispose = undefined
}

const resume = (root: HTMLElement) =>
  root.querySelector<HTMLButtonElement>('[data-testid="repair-resume-setup"]')
const cancelPending = (root: HTMLElement) =>
  root.querySelector<HTMLButtonElement>('[data-testid="repair-cancel-pending"]')

it("a pending setup that can't be cancelled is discarded by Confirm, against the setup shown", () => {
  setPending(pendingStatus({}))
  const root = mount()
  expect(cancelPending(root)).toBeNull()
  expect(confirm(root).disabled).toBe(false)
  expect(confirm(root).textContent).toContain("_trn_repair_confirm_discard")
  // Nothing is withheld: the preview of the installed record still shows.
  expect(root.querySelector('[data-testid="repair-preview-counts"]')).not.toBe(
    null
  )
  confirm(root).click()
  expect(c.mutate).toHaveBeenCalledWith("instance.repairModpack", {
    instance: 7,
    cleanup_paths: [],
    cleanup_selections: [],
    re_enable_disabled: false,
    expected_pending: "op-1"
  })
  expect(c.close).toHaveBeenCalledTimes(1)
})

it("a setup this instance doesn't own is discarded as legacy, and nothing pending sends null", () => {
  setPending(
    pendingStatus({
      operation: null,
      can_resume: false,
      discard_token: "legacy"
    })
  )
  const root = mount()
  expect(resume(root)).toBeNull()
  confirm(root).click()
  expect(c.mutate).toHaveBeenLastCalledWith(
    "instance.repairModpack",
    expect.objectContaining({ expected_pending: "legacy" })
  )
  dispose?.()
  dispose = undefined
  setPending({
    data: { pending: false, operation: null, discard_token: null },
    isSuccess: true,
    isFetching: false,
    isError: false
  })
  const other = mount()
  expect(confirm(other).textContent).toContain("_trn_repair_confirm")
  expect(confirm(other).textContent).not.toContain(
    "_trn_repair_confirm_discard"
  )
  confirm(other).click()
  expect(c.mutate).toHaveBeenLastCalledWith(
    "instance.repairModpack",
    expect.objectContaining({ expected_pending: null })
  )
})

it("a withheld preview is never fetched, and Confirm sends no file selections", () => {
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
  // The setup turns out to have stopped while changing the pack's files.
  setPending(pendingStatus({ preview_withheld: true, repair_version: "v2" }))
  setPreview({ ...ready(), data: undefined, isSuccess: false, isError: true })
  expect(
    c.queryKeys
      .filter((f) => f().queryKey[0] === "instance.getRepairPreview")
      .every((f) => f().enabled === false)
  ).toBe(true)
  expect(root.querySelector('[data-testid="repair-preview-counts"]')).toBe(null)
  expect(root.textContent).not.toContain("repair_preview_failed")
  expect(root.textContent).toContain("_trn_repair_pending_withheld")
  expect(confirm(root).disabled).toBe(false)
  confirm(root).click()
  expect(c.mutate).toHaveBeenCalledWith("instance.repairModpack", {
    instance: 7,
    cleanup_paths: [],
    cleanup_selections: [],
    re_enable_disabled: false,
    expected_pending: "op-1"
  })
})

it("Resume setup runs the instance's own preparation", () => {
  setPending(pendingStatus({}))
  const root = mount()
  resume(root)!.click()
  expect(c.mutate).toHaveBeenCalledWith("instance.prepareInstance", 7)
  expect(c.mutate).not.toHaveBeenCalledWith(
    "instance.repairModpack",
    expect.anything()
  )
})

it("an eligible change can still be cancelled against the operation shown", () => {
  setPending(pendingStatus({ can_cancel: true }))
  const root = mount()
  expect(root.textContent).toContain("_trn_repair_pending_alternatives")
  cancelPending(root)!.click()
  expect(c.mutate).toHaveBeenCalledWith("instance.cancelPendingSetup", {
    instance: 7,
    expected_operation: "op-1"
  })
})

it("every pending-setup string comes from a translation key", () => {
  setPending(pendingStatus({ preview_withheld: true }))
  const root = mount()
  const text = root.textContent ?? ""
  for (const key of [
    "_trn_repair_pending_title",
    "_trn_repair_pending_withheld",
    "_trn_repair_pending_resume_hint",
    "_trn_repair_pending_resume",
    "_trn_repair_confirm_discard"
  ])
    expect(text).toContain(key)
  for (const english of [
    "A setup is pending",
    "Installed version",
    "Pending version",
    "Cancel pending change",
    "A reason the dialog no longer shows"
  ])
    expect(text).not.toContain(english)
  dispose?.()
  dispose = undefined
  setPending({
    data: undefined,
    isSuccess: false,
    isError: true,
    isFetching: false
  })
  const failed = mount()
  expect(failed.textContent).toContain("_trn_repair_pending_status_failed")
  expect(failed.textContent).not.toContain("Pending setup status")
})

it("names the version Repair installs, never by its raw id", () => {
  c.modpackInfo = { data: { version_name: "1.2.3" } }
  setPending(pendingStatus({ repair_version: "v1" }))
  const root = mount()
  expect(root.textContent).toContain("_trn_repair_pending_discard(1.2.3)")
  unmount()
  // Offline, the configured version has no name.
  c.modpackInfo = { data: undefined }
  const offline = mount()
  expect(offline.textContent).toContain("_trn_repair_pending_discard_unnamed")
  expect(offline.textContent).not.toContain("(v1)")
  unmount()
  // An interrupted update installs the version it was applying, by its
  // platform name, and not "again".
  c.mrVersion = (id: string) =>
    id === "v2" ? { id: "v2", version_number: "2.0.0" } : undefined
  setPending(pendingStatus({ repair_version: "v2", preview_withheld: true }))
  const other = mount()
  expect(other.textContent).toContain(
    "_trn_repair_pending_withheld_target(2.0.0)"
  )
  expect(other.textContent).not.toContain("_trn_repair_pending_withheld(")
  unmount()
  // Without its name, a sentence that needs none.
  c.mrVersion = () => undefined
  const unnamed = mount()
  expect(unnamed.textContent).toContain(
    "_trn_repair_pending_withheld_target_unnamed"
  )
  expect(unnamed.textContent).not.toContain("v2")
  unmount()
  // The configured version, withheld, is installed again.
  setPending(pendingStatus({ repair_version: "v1", preview_withheld: true }))
  const again = mount()
  expect(again.textContent).toContain("_trn_repair_pending_withheld_unnamed")
})

it("names a CurseForge target by its file's display name", () => {
  setDetails({
    data: {
      modpack: {
        modpack: {
          type: "curseforge",
          value: { project_id: 11, file_id: 100 }
        }
      }
    }
  })
  c.cfFile = (file: { modId: number; fileId: number }) =>
    file.modId === 11 && file.fileId === 200
      ? { data: { id: 200, displayName: "Pack 2.0" } }
      : undefined
  setPending(pendingStatus({ repair_version: "200", preview_withheld: true }))
  const root = mount()
  expect(root.textContent).toContain(
    "_trn_repair_pending_withheld_target(Pack 2.0)"
  )
})

it("shows a version name with an apostrophe, an ampersand or a slash as written", () => {
  c.modpackInfo = { data: { version_name: "Farmer's & Co 1.2/Forge" } }
  setPending(pendingStatus({ repair_version: "v1" }))
  const root = mount()
  expect(root.textContent).toContain(
    "_trn_repair_pending_discard(Farmer's & Co 1.2/Forge)"
  )
  expect(root.textContent).not.toContain("&#39;")
  expect(root.textContent).not.toContain("&amp;")
  expect(root.textContent).not.toContain("&#x2F;")
})

it("names the version a file shipped in as written", () => {
  const state = ready()
  state.data.untracked = [
    {
      ...file,
      origin: {
        ShippedIn: { version_name: "Farmer's & Co 1.2/Forge", version_id: "v0" }
      }
    } as any
  ]
  setPreview(state)
  const root = mount()
  const row = root.querySelector('[data-testid="repair-untracked-row"]')!
  expect(row.textContent).toContain(
    "_trn_repair_origin_shipped_in(Farmer's & Co 1.2/Forge)"
  )
  expect(row.textContent).not.toContain("&#39;")
  expect(row.textContent).not.toContain("&amp;")
  expect(row.textContent).not.toContain("&#x2F;")
})

it("sends back the token the core gave, not one derived from the status", () => {
  setPending(
    pendingStatus({ operation: "op-1", discard_token: "token-from-core" })
  )
  const root = mount()
  confirm(root).click()
  expect(c.mutate).toHaveBeenCalledWith(
    "instance.repairModpack",
    expect.objectContaining({ expected_pending: "token-from-core" })
  )
})

it("while an install or update is still running, says so and offers nothing to click", () => {
  setPending(
    pendingStatus({ busy: true, can_resume: false, can_cancel: false })
  )
  const root = mount()
  const text = root.textContent ?? ""
  expect(text).toContain("_trn_repair_pending_busy_title")
  expect(text).toMatch(/_trn_repair_pending_busy(?!_title)/)
  expect(text).not.toContain("_trn_repair_pending_title")
  expect(text).not.toContain("_trn_repair_confirm_discard")
  expect(resume(root)).toBeNull()
  expect(cancelPending(root)).toBeNull()
  // Nothing is previewed while its files are being installed.
  expect(
    c.queryKeys
      .filter((f) => f().queryKey[0] === "instance.getRepairPreview")
      .every((f) => f().enabled === false)
  ).toBe(true)
  expect(root.querySelector('[data-testid="repair-preview-counts"]')).toBe(null)
  expect(root.querySelector('[data-testid="repair-reenable-checkbox"]')).toBe(
    null
  )
  // Nothing below to tick, so nothing points at an option or files below.
  expect(text).toContain("_trn_repair_replaced_instance_withheld")
  expect(text).toContain("_trn_repair_kept_instance_busy")
  expect(text).not.toMatch(/_trn_repair_replaced_instance(?!_withheld)/)
  expect(text).not.toMatch(/_trn_repair_kept_instance(?!_busy)/)
  expect(confirm(root).disabled).toBe(true)
  void (confirm(root) as any).invoke()
  expect(c.close).not.toHaveBeenCalled()
  expect(c.navigate).not.toHaveBeenCalled()
  expect(c.mutate).not.toHaveBeenCalled()
  // Once it stops, Repair can discard it, and the preview's options return.
  setPending(pendingStatus({}))
  expect(confirm(root).disabled).toBe(false)
  const stopped = root.textContent ?? ""
  expect(stopped).toContain("_trn_repair_pending_title")
  expect(stopped).not.toContain("_trn_repair_kept_instance_busy")
  expect(stopped).not.toContain("_withheld")
})

it("while the instance is launching or running, Repair waits with the reason shown", () => {
  setPending({
    data: { pending: false, busy: true, discard_token: null },
    isSuccess: true,
    isFetching: false,
    isError: false
  })
  const root = mount()
  expect(root.querySelector('[data-testid="repair-busy"]')?.textContent).toBe(
    "instances:_trn_repair_busy"
  )
  // The preview still shows what Repair will do.
  expect(root.querySelector('[data-testid="repair-preview-counts"]')).not.toBe(
    null
  )
  expect(confirm(root).disabled).toBe(true)
  void (confirm(root) as any).invoke()
  expect(c.mutate).not.toHaveBeenCalled()
})

it("without a preview, never points at options or files below", () => {
  setPending(pendingStatus({ preview_withheld: true }))
  const root = mount()
  const text = root.textContent ?? ""
  expect(text).toContain("_trn_repair_replaced_instance_withheld")
  expect(text).toContain("_trn_repair_kept_instance_withheld")
  expect(text).not.toMatch(/_trn_repair_replaced_instance(?!_withheld)/)
  expect(text).not.toMatch(/_trn_repair_kept_instance(?!_withheld)/)
  expect(root.querySelector('[data-testid="repair-reenable-checkbox"]')).toBe(
    null
  )
  unmount()
  setPending(pendingStatus({}))
  const shown = mount()
  const visible = shown.textContent ?? ""
  expect(visible).not.toContain("_withheld")
  expect(visible).toContain("_trn_repair_replaced_instance")
  expect(visible).toContain("_trn_repair_kept_instance")
})

// What a refused mutation throws: the rspc body, whose message is the core's
// error with its debug text and backtrace.
const refusal = (display: string) =>
  new Error(
    JSON.stringify({
      code: 500,
      message: JSON.stringify(
        { cause: [{ display, debug: "Debug" }], backtrace: "0: make_rspc" },
        null,
        2
      )
    })
  )

it.each([
  {
    key: "instance.prepareInstance",
    title: "instances:_trn_repair_pending_resume_failed",
    act: () => {
      setPending(pendingStatus({}))
      resume(mount())!.click()
    }
  },
  {
    key: "instance.cancelPendingSetup",
    title: "instances:_trn_repair_pending_cancel_failed",
    act: () => {
      setPending(pendingStatus({ can_cancel: true }))
      cancelPending(mount())!.click()
    }
  },
  {
    key: "instance.repairModpack",
    title: "instances:_trn_repair_failed",
    act: () => confirm(mount()).click()
  },
  {
    key: "server.reinstallServer",
    title: "instances:_trn_repair_failed",
    act: () => confirm(mount(true)).click()
  }
])("a refused $key shows one toast with the core's sentence", (step) => {
  c.failing[step.key] = refusal("Repair was refused for a plain reason")
  step.act()
  expect(c.mutate).toHaveBeenCalledWith(step.key, expect.anything())
  expect(toast.error).toHaveBeenCalledTimes(1)
  const [title, options] = (toast.error as any).mock.calls[0]
  expect(title).toBe(step.title)
  expect(options.description).toBe("Repair was refused for a plain reason")
  expect(options.description).not.toContain("{")
  expect(options.action.label).toBe("general:_trn_copy")
  // The global error toast leaves the cause to this one.
  expect(c.options[step.key].meta?.localErrorToast).toBe(true)
})
