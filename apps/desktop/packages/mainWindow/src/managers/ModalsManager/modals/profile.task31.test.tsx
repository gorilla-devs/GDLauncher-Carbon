import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import { createStore } from "solid-js/store"
import { button, deferred, input, settle, toast } from "@/task31.fixture"
import EditGDLProfile from "./EditGDLProfile"
import Accounts from "@/pages/Settings/Accounts"

const c = vi.hoisted(() => ({
  store: undefined as any,
  peek: undefined as any,
  mutate: vi.fn(),
  close: vi.fn(),
  invalidate: vi.fn(),
  refetch: vi.fn()
}))
vi.mock("@gd/ui", async () => import("@/task31.fixture"))
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => <span>{p.key}</span>,
  useTransContext: () => [(k: string) => k]
}))
vi.mock("@/components/GlobalStoreContext", () => ({
  useGlobalStore: () => c.store
}))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ closeModal: c.close, openModal: vi.fn() })
}))
vi.mock("@/managers/ModalsManager/ModalLayout", () => ({
  default: (p: any) => <div>{p.children}</div>
}))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ navigate: vi.fn() })
}))
vi.mock("@/components/ImagePicker", () => ({
  default: (p: any) => (
    <>
      <button onClick={() => p.onDelete()}>remove avatar</button>
      <button onClick={() => p.onSelect("synthetic-image")}>
        select avatar
      </button>
    </>
  )
}))
vi.mock("@/utils/helpers", () => ({
  convertSecondsToHumanTime: (n: number) => String(n),
  blobToBase64: async () => "data:image/png;base64,synthetic"
}))
vi.mock("@/components/SharePreviewContent", () => ({
  getErrorCode: () => null
}))
vi.mock("@/utils/showcaseHelpers", () => ({
  getAccountImageUuid: () => "synthetic"
}))
vi.mock("@/utils/rspcClient", () => ({
  apiUrl: (v: string) => v,
  apiFetch: async () => ({ blob: async () => ({}) }),
  queryClient: {
    invalidateQueries: c.invalidate,
    getQueryData: ([_key, id]: any) => c.peek[id]?.data
  },
  rspc: {
    createMutation: (f: any) => ({
      mutateAsync: (v: any) => c.mutate(f().mutationKey[0], v),
      mutate: vi.fn(),
      isPending: false
    }),
    createQuery: (f: any) => ({
      get data() {
        const [key, id] = f().queryKey
        return key === "account.peekGdlAccount"
          ? c.peek[id]?.data
          : [{ displayName: "Historical", changedAt: "2020-01-01" }]
      },
      get isPending() {
        return !!c.peek[f().queryKey[1]]?.isPending
      },
      get isFetching() {
        return !!c.peek[f().queryKey[1]]?.isFetching
      },
      get isError() {
        return !!c.peek[f().queryKey[1]]?.isError
      },
      refetch: () => c.refetch(f().queryKey[1])
    })
  }
}))
const user = (id = "A") => ({
  microsoftOid: `oid${id}`,
  friendCode: `friend${id}`,
  displayName: `Owner${id}`,
  email: `${id}@synthetic.invalid`,
  profileIconUrl: "avatar",
  hasCustomAvatar: true,
  isEmailVerified: false,
  displayNameChangeTimeout: null,
  verificationTimeout: null,
  emailChangeTimeout: null
})
let dispose: (() => void) | undefined, setStore: any, setPeek: any
beforeEach(() => {
  vi.clearAllMocks()
  c.mutate.mockReset()
  c.mutate.mockResolvedValue({ status: "success" })
  c.refetch.mockReset()
  c.refetch.mockResolvedValue(undefined)
  const [store, set] = createStore({
    settings: { data: { gdlAccountId: "A" } },
    gdlAccount: { data: { status: "valid", value: user() }, isError: false },
    currentlySelectedAccountUuid: { data: "A" },
    accounts: { data: [] }
  })
  c.store = store
  setStore = set
  const [peek, setP] = createStore({
    A: { data: user(), isPending: false, isFetching: false, isError: false },
    B: { data: user("B"), isPending: false, isFetching: false, isError: false }
  })
  c.peek = peek
  setPeek = setP
})
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
  vi.useRealTimers()
})
function mount(page = false) {
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(() => (page ? <Accounts /> : <EditGDLProfile />), root)
  return root
}
const save = (root: HTMLElement) => button(root, "save_changes")
it.each(["B", null])(
  "0034 saved displayed A owns name/email/verification/avatar with active %j",
  async (active) => {
    setStore("currentlySelectedAccountUuid", "data", active)
    const root = mount()
    input(root, 0, "New owner name")
    input(root, 1, "new@synthetic.invalid")
    button(root, "remove avatar").click()
    save(root).click()
    await settle()
    expect(c.mutate.mock.calls).toEqual([
      ["account.deleteProfileIcon", "A"],
      [
        "account.changeGdlAccountDisplayName",
        { uuid: "A", displayName: "New owner name" }
      ],
      [
        "account.requestEmailChange",
        { uuid: "A", email: "new@synthetic.invalid" }
      ]
    ])
    expect(c.close).toHaveBeenCalledTimes(1)
  }
)
it("0034 verification uses saved A independently of active B", async () => {
  setStore("currentlySelectedAccountUuid", "data", "B")
  const root = mount()
  button(root, "verify_email").click()
  await settle()
  expect(c.mutate).toHaveBeenCalledWith(
    "account.requestNewVerificationToken",
    "A"
  )
})
it("0034 avatar upload uses captured saved A and successful name/email values remain immutable across await", async () => {
  const wait = deferred()
  c.mutate
    .mockImplementationOnce(() => wait.promise)
    .mockResolvedValue({ status: "success" })
  const root = mount()
  button(root, "select avatar").click()
  await settle()
  input(root, 0, "Submitted name")
  input(root, 1, "submitted@synthetic.invalid")
  save(root).click()
  input(root, 0, "Later unsaved name")
  input(root, 1, "later@synthetic.invalid")
  setStore("currentlySelectedAccountUuid", "data", "B")
  wait.resolve({ status: "success" })
  await settle()
  expect(c.mutate.mock.calls).toEqual([
    ["account.uploadProfileIcon", { uuid: "A", iconPath: "synthetic-image" }],
    [
      "account.changeGdlAccountDisplayName",
      { uuid: "A", displayName: "Submitted name" }
    ],
    [
      "account.requestEmailChange",
      { uuid: "A", email: "submitted@synthetic.invalid" }
    ]
  ])
  expect(c.close).not.toHaveBeenCalled()
  expect(root.querySelectorAll("input")[0].value).toBe("Later unsaved name")
  expect(save(root).disabled).toBe(false)
  save(root).click()
  await settle()
  expect(c.mutate.mock.calls.slice(3)).toEqual([
    [
      "account.changeGdlAccountDisplayName",
      { uuid: "A", displayName: "Later unsaved name" }
    ],
    [
      "account.requestEmailChange",
      { uuid: "A", email: "later@synthetic.invalid" }
    ]
  ])
  expect(c.close).toHaveBeenCalledTimes(1)
})
it.each(["removed", "replaced", "ABA"])(
  "0034 saved-link %s revokes continuation after name await",
  async (kind) => {
    const wait = deferred()
    c.mutate.mockReturnValue(wait.promise)
    const root = mount()
    input(root, 0, "Submitted name")
    input(root, 1, "submitted@synthetic.invalid")
    save(root).click()
    setStore(
      "settings",
      "data",
      "gdlAccountId",
      kind === "removed" ? null : "B"
    )
    if (kind === "ABA") setStore("settings", "data", "gdlAccountId", "A")
    wait.resolve({ status: "success" })
    await settle()
    expect(c.mutate).toHaveBeenCalledTimes(1)
    expect(c.close).not.toHaveBeenCalled()
    expect(save(root).getAttribute("data-loading")).toBe("false")
  }
)
it.each(["pending", "error", "mismatch"])(
  "0034 initial settings B/displayed A with %s keyed proof refuses action visibly",
  async (kind) => {
    setStore("settings", "data", "gdlAccountId", "B")
    if (kind === "pending") {
      setPeek("B", "data", undefined)
      setPeek("B", "isPending", true)
    }
    if (kind === "error") setPeek("B", "isError", true)
    const root = mount()
    input(root, 0, "Edited name")
    ;(save(root) as any).invoke()
    button(root, "verify_email").click()
    await settle()
    expect(c.mutate).not.toHaveBeenCalled()
    // An unanswered first check is reported as loading; only a definite
    // error or mismatch is reported as unavailable.
    if (kind === "pending") {
      expect(root.textContent).toContain("checking_gdl_account")
      expect(root.textContent).not.toContain("gdl_account_unavailable")
    } else expect(root.textContent).toContain("gdl_account_unavailable")
  }
)
it("0034 delayed matching keyed profile admits owner without requiring active account", async () => {
  setPeek("A", "data", undefined)
  setPeek("A", "isPending", true)
  setStore("currentlySelectedAccountUuid", "data", null)
  const root = mount()
  setPeek("A", "data", user())
  setPeek("A", "isPending", false)
  input(root, 0, "Delayed owner name")
  save(root).click()
  await settle()
  expect(c.mutate).toHaveBeenCalledWith("account.changeGdlAccountDisplayName", {
    uuid: "A",
    displayName: "Delayed owner name"
  })
})
it.each([null, 0, 30])(
  "0107 failed name status with cooldown %j stays open, preserves draft and displays field failure",
  async (value) => {
    vi.useFakeTimers()
    c.mutate.mockResolvedValue({ status: "failed", value })
    const root = mount()
    input(root, 0, "Attempted name")
    save(root).click()
    await settle()
    expect(c.close).not.toHaveBeenCalled()
    expect(root.querySelector("input")?.value).toBe("Attempted name")
    expect(root.textContent).toContain("display_name_change_failed")
    expect(save(root).getAttribute("data-loading")).toBe("false")
    expect(vi.getTimerCount()).toBe(value === 30 ? 1 : 0)
  }
)
it.each([null, 0, 30])(
  "0107 failed email status with cooldown %j stays open and displays field failure",
  async (value) => {
    c.mutate.mockResolvedValue({ status: "failed", value })
    const root = mount()
    input(root, 1, "attempt@synthetic.invalid")
    save(root).click()
    await settle()
    expect(c.close).not.toHaveBeenCalled()
    expect(root.querySelectorAll("input")[1].value).toBe(
      "attempt@synthetic.invalid"
    )
    expect(root.textContent).toContain("email_change_failed")
    expect(save(root).getAttribute("data-loading")).toBe("false")
  }
)
it("0107 name/avatar success then email failure retries only remaining email despite old profile refetch", async () => {
  c.mutate
    .mockResolvedValueOnce({ status: "success" })
    .mockResolvedValueOnce({ status: "success" })
    .mockResolvedValueOnce({ status: "failed", value: 0 })
    .mockResolvedValue({ status: "success" })
  const root = mount()
  button(root, "remove avatar").click()
  input(root, 0, "Changed name")
  input(root, 1, "changed@synthetic.invalid")
  save(root).click()
  await settle()
  expect(c.close).not.toHaveBeenCalled()
  setStore("gdlAccount", "data", "value", { ...user() })
  save(root).click()
  await settle()
  expect(c.mutate.mock.calls.map(([key]) => key)).toEqual([
    "account.deleteProfileIcon",
    "account.changeGdlAccountDisplayName",
    "account.requestEmailChange",
    "account.requestEmailChange"
  ])
  expect(c.invalidate).toHaveBeenCalledWith({
    queryKey: ["account.getDisplayNameHistory", "friendA"]
  })
  expect(c.close).toHaveBeenCalledTimes(1)
})
it("0107 pending reentry is blocked, rejected RPC resets loading and preserves retry without extra global toast", async () => {
  const wait = deferred()
  c.mutate
    .mockReturnValueOnce(wait.promise)
    .mockResolvedValue({ status: "success" })
  const root = mount()
  input(root, 0, "Changed name")
  save(root).click()
  ;(save(root) as any).invoke()
  expect(c.mutate).toHaveBeenCalledTimes(1)
  wait.reject(new Error("synthetic RPC rejection"))
  await settle()
  expect(save(root).getAttribute("data-loading")).toBe("false")
  expect(c.close).not.toHaveBeenCalled()
  expect(toast.error).not.toHaveBeenCalled()
  save(root).click()
  await settle()
  expect(c.close).toHaveBeenCalledTimes(1)
})
it("0107 invalid name cannot bypass disabled save handler", async () => {
  const root = mount()
  input(root, 0, "bad")
  expect(save(root).disabled).toBe(true)
  ;(save(root) as any).invoke()
  await settle()
  expect(c.mutate).not.toHaveBeenCalled()
  expect(c.close).not.toHaveBeenCalled()
})
it("0034 actual Accounts history clears displayed saved owner and invalidates captured friend code", async () => {
  setStore("currentlySelectedAccountUuid", "data", "B")
  const root = mount(true)
  button(root, "clear_display_name_history").click()
  await settle()
  expect(c.mutate).toHaveBeenCalledWith("account.clearDisplayNameHistory", "A")
  expect(c.invalidate).toHaveBeenCalledWith({
    queryKey: ["account.getDisplayNameHistory", "friendA"]
  })
})
it("0034 history promise cannot invalidate replacement or show stale success after link revocation", async () => {
  const wait = deferred()
  c.mutate.mockReturnValue(wait.promise)
  const root = mount(true)
  button(root, "clear_display_name_history").click()
  setStore("settings", "data", "gdlAccountId", "B")
  setStore("gdlAccount", "data", "value", user("B"))
  wait.resolve({ status: "success" })
  await settle()
  expect(c.invalidate).not.toHaveBeenCalled()
  expect(toast.success).not.toHaveBeenCalled()
})

it("0107 successful avatar invalidation starts stable-owner delayed peek refetch without interrupting multi-field save", async () => {
  c.mutate.mockImplementation(async (key) => {
    if (key === "account.deleteProfileIcon") setPeek("A", "isFetching", true)
    return { status: "success" }
  })
  const root = mount()
  button(root, "remove avatar").click()
  input(root, 0, "Changed name")
  input(root, 1, "changed@synthetic.invalid")
  save(root).click()
  await settle()
  expect(c.mutate.mock.calls.map(([key]) => key)).toEqual([
    "account.deleteProfileIcon",
    "account.changeGdlAccountDisplayName",
    "account.requestEmailChange"
  ])
  expect(c.close).toHaveBeenCalledTimes(1)
})
it("0107 successful name invalidation with delayed/error peek refetch records baseline before failed email retry", async () => {
  c.mutate.mockImplementation(async (key) => {
    if (key === "account.changeGdlAccountDisplayName") {
      setPeek("A", "isFetching", true)
      return { status: "success" }
    }
    return { status: "failed", value: 0 }
  })
  const root = mount()
  input(root, 0, "Changed name")
  input(root, 1, "changed@synthetic.invalid")
  save(root).click()
  await settle()
  expect(c.mutate.mock.calls.map(([key]) => key)).toEqual([
    "account.changeGdlAccountDisplayName",
    "account.requestEmailChange"
  ])
  setPeek("A", "isFetching", false)
  setPeek("A", "isError", true)
  c.mutate.mockResolvedValue({ status: "success" })
  save(root).click()
  await settle()
  expect(c.mutate.mock.calls.map(([key]) => key)).toEqual([
    "account.changeGdlAccountDisplayName",
    "account.requestEmailChange",
    "account.requestEmailChange"
  ])
  expect(c.close).toHaveBeenCalledTimes(1)
})

it("0034 saved=active normal success closes once and profile refetch cannot replace typed text", async () => {
  const root = mount()
  input(root, 0, "Normal changed name")
  input(root, 1, "normal@synthetic.invalid")
  setStore("gdlAccount", "data", "value", { ...user() })
  expect(root.querySelectorAll("input")[0].value).toBe("Normal changed name")
  save(root).click()
  await settle()
  expect(c.mutate.mock.calls).toEqual([
    [
      "account.changeGdlAccountDisplayName",
      { uuid: "A", displayName: "Normal changed name" }
    ],
    [
      "account.requestEmailChange",
      { uuid: "A", email: "normal@synthetic.invalid" }
    ]
  ])
  expect(c.close).toHaveBeenCalledTimes(1)
})
it.each(["null", "mismatch", "lost"])(
  "0034 definite keyed proof %s then restoration cannot resurrect an awaiting old owner",
  async (kind) => {
    const wait = deferred()
    c.mutate.mockReturnValue(wait.promise)
    const root = mount()
    input(root, 0, "Submitted name")
    input(root, 1, "submitted@synthetic.invalid")
    save(root).click()
    setPeek(
      "A",
      "data",
      kind === "null" ? null : kind === "lost" ? undefined : user("B")
    )
    setPeek("A", "data", user())
    wait.resolve({ status: "success" })
    await settle()
    expect(c.mutate).toHaveBeenCalledTimes(1)
    expect(c.close).not.toHaveBeenCalled()
    expect(save(root).disabled).toBe(true)
  }
)

// solid-query replaces `data` wholesale on every refetch (structuralSharing is
// off and no reconcile is set), so these tests replace `data` too instead of
// merging into `data.value`.
const refetchProfile = (id = "A", over: Record<string, unknown> = {}) => {
  setStore("gdlAccount", {
    data: { status: "valid", value: { ...user(id), ...over } }
  })
  setStore("settings", { data: { gdlAccountId: "A" } })
}
const fields = (root: HTMLElement) => Array.from(root.querySelectorAll("input"))
const expectUsable = (root: HTMLElement) => {
  expect(root.textContent).not.toContain("gdl_account_unavailable")
  expect(root.textContent).not.toContain("gdl_account_changed_reopen")
  expect(fields(root).map((e) => e.disabled)).toEqual([false, false])
}
const expectLocked = (root: HTMLElement) => {
  expect(fields(root).map((e) => e.disabled)).toEqual([true, true])
  expect(save(root).disabled).toBe(true)
}

it("edit-profile Verify email refetch that replaces the profile wholesale leaves fields and Save enabled", async () => {
  c.mutate.mockImplementation(async (key) => {
    if (key === "account.requestNewVerificationToken") {
      refetchProfile()
      setPeek("A", { data: user() })
    }
    return { status: "success" }
  })
  const root = mount()
  button(root, "verify_email").click()
  await settle()
  expect(c.mutate).toHaveBeenCalledWith(
    "account.requestNewVerificationToken",
    "A"
  )
  expectUsable(root)
  input(root, 0, "After verify name")
  expect(save(root).disabled).toBe(false)
  save(root).click()
  await settle()
  expect(c.mutate).toHaveBeenLastCalledWith(
    "account.changeGdlAccountDisplayName",
    { uuid: "A", displayName: "After verify name" }
  )
  expect(c.close).toHaveBeenCalledTimes(1)
})

it("edit-profile avatar+name+email save survives the avatar refetch landing during the name await", async () => {
  const nameWait = deferred()
  c.mutate.mockImplementation((key) =>
    key === "account.changeGdlAccountDisplayName"
      ? nameWait.promise
      : Promise.resolve({ status: "success" })
  )
  const root = mount()
  button(root, "select avatar").click()
  await settle()
  input(root, 0, "Three field name")
  input(root, 1, "three@synthetic.invalid")
  save(root).click()
  await settle()
  // The upload invalidated both queries; their refetch lands now.
  refetchProfile("A", { profileIconUrl: "new-avatar" })
  setPeek("A", { isFetching: false, data: user() })
  nameWait.resolve({ status: "success" })
  await settle()
  expect(c.mutate.mock.calls).toEqual([
    ["account.uploadProfileIcon", { uuid: "A", iconPath: "synthetic-image" }],
    [
      "account.changeGdlAccountDisplayName",
      { uuid: "A", displayName: "Three field name" }
    ],
    [
      "account.requestEmailChange",
      { uuid: "A", email: "three@synthetic.invalid" }
    ]
  ])
  expect(c.close).toHaveBeenCalledTimes(1)
})

it("edit-profile email failure followed by a wholesale refetch keeps the inputs enabled for a retry", async () => {
  c.mutate
    .mockResolvedValueOnce({ status: "failed", value: 0 })
    .mockResolvedValue({ status: "success" })
  const root = mount()
  input(root, 1, "retry@synthetic.invalid")
  save(root).click()
  await settle()
  expect(root.textContent).toContain("email_change_failed")
  // The core invalidates the profile after a failed email change too.
  refetchProfile()
  setPeek("A", { data: user() })
  await settle()
  expectUsable(root)
  expect(fields(root)[1].value).toBe("retry@synthetic.invalid")
  save(root).click()
  await settle()
  expect(c.mutate.mock.calls.map(([key]) => key)).toEqual([
    "account.requestEmailChange",
    "account.requestEmailChange"
  ])
  expect(c.close).toHaveBeenCalledTimes(1)
})

it("edit-profile opened while the first keyed check is pending is usable once it resolves flags-first", async () => {
  setPeek("A", { data: undefined, isPending: true, isFetching: true })
  const root = mount()
  expect(root.textContent).toContain("checking_gdl_account")
  expect(root.textContent).not.toContain("gdl_account_unavailable")
  // query-core writes isPending before data in one merge.
  setPeek("A", { isPending: false, isFetching: false, data: user() })
  await settle()
  expect(root.textContent).not.toContain("checking_gdl_account")
  expectUsable(root)
  expect(fields(root)[0].value).toBe("OwnerA")
  input(root, 0, "Resolved owner name")
  save(root).click()
  await settle()
  expect(c.mutate).toHaveBeenCalledWith("account.changeGdlAccountDisplayName", {
    uuid: "A",
    displayName: "Resolved owner name"
  })
  expect(c.close).toHaveBeenCalledTimes(1)
})

it("edit-profile opened with a cached keyed check that is refetching is populated and usable, and a B answer locks it", async () => {
  setPeek("A", "isFetching", true)
  const root = mount()
  expectUsable(root)
  expect(fields(root)[0].value).toBe("OwnerA")
  expect(fields(root)[1].value).toBe("A@synthetic.invalid")
  input(root, 0, "While refetching")
  expect(save(root).disabled).toBe(false)
  setPeek("A", { isFetching: false, data: user("B") })
  await settle()
  expect(root.textContent).toContain("gdl_account_unavailable")
  expectLocked(root)
  ;(save(root) as any).invoke()
  await settle()
  expect(c.mutate).not.toHaveBeenCalled()
})

it("edit-profile ABA locks the dialog and Retry with the same account unlocks it keeping typed text", async () => {
  const root = mount()
  input(root, 0, "Typed before ABA")
  setPeek("A", "data", user("B"))
  setPeek("A", "data", user())
  await settle()
  expect(root.textContent).toContain("gdl_account_unavailable")
  expectLocked(root)
  button(root, "general:_trn_retry").click()
  await settle()
  expect(c.refetch).toHaveBeenCalledWith("A")
  expectUsable(root)
  expect(fields(root)[0].value).toBe("Typed before ABA")
  save(root).click()
  await settle()
  expect(c.mutate).toHaveBeenCalledWith("account.changeGdlAccountDisplayName", {
    uuid: "A",
    displayName: "Typed before ABA"
  })
  expect(c.close).toHaveBeenCalledTimes(1)
})

it("edit-profile Retry keeps the lock when the keyed check now names another account", async () => {
  const root = mount()
  input(root, 0, "Typed for A")
  setPeek("A", "data", user("B"))
  await settle()
  button(root, "general:_trn_retry").click()
  await settle()
  expect(c.refetch).toHaveBeenCalledWith("A")
  expect(root.textContent).toContain("gdl_account_unavailable")
  expectLocked(root)
  // A second Retry whose answer arrives after the displayed profile became B
  // as well must not admit B into a form typed for A.
  const wait = deferred()
  c.refetch.mockReturnValueOnce(wait.promise)
  button(root, "general:_trn_retry").click()
  refetchProfile("B")
  wait.resolve(undefined)
  await settle()
  // Retry cannot help any more: say so instead of offering it.
  expect(root.textContent).toContain("gdl_account_changed_reopen")
  expect(root.textContent).not.toContain("general:_trn_retry")
  expectLocked(root)
  expect(fields(root)[0].value).toBe("Typed for A")
  expect(c.mutate).not.toHaveBeenCalled()
})

it("edit-profile switching the saved link to another account asks to reopen the dialog", async () => {
  const root = mount()
  input(root, 0, "Typed for A")
  setStore("settings", { data: { gdlAccountId: "B" } })
  setStore("gdlAccount", { data: { status: "valid", value: user("B") } })
  await settle()
  expect(root.textContent).toContain("gdl_account_changed_reopen")
  expect(root.textContent).not.toContain("general:_trn_retry")
  expectLocked(root)
  expect(fields(root)[0].value).toBe("Typed for A")
})

it.each([false, true])(
  "edit-profile pending first keyed check shows loading, not the warning; an error shows the warning (Accounts page %s)",
  async (page) => {
    setPeek("A", { data: undefined, isPending: true, isFetching: true })
    const root = mount(page)
    expect(root.textContent).toContain("checking_gdl_account")
    expect(root.textContent).not.toContain("gdl_account_unavailable")
    setPeek("A", { isPending: false, isFetching: false, isError: true })
    await settle()
    expect(root.textContent).not.toContain("checking_gdl_account")
    expect(root.textContent).toContain("gdl_account_unavailable")
    button(root, "general:_trn_retry").click()
    await settle()
    expect(c.refetch).toHaveBeenCalledWith("A")
  }
)

it("edit-profile Accounts page with a cached keyed check that is refetching shows no warning and keeps history clearing", async () => {
  setPeek("A", "isFetching", true)
  const root = mount(true)
  expect(root.textContent).not.toContain("gdl_account_unavailable")
  expect(root.textContent).not.toContain("checking_gdl_account")
  expect(button(root, "clear_display_name_history").disabled).toBe(false)
  refetchProfile()
  setPeek("A", { isFetching: false, data: user() })
  await settle()
  expect(root.textContent).not.toContain("gdl_account_unavailable")
  button(root, "clear_display_name_history").click()
  await settle()
  expect(c.mutate).toHaveBeenCalledWith("account.clearDisplayNameHistory", "A")
})

it.each([
  [
    "became invalid",
    () => setStore("gdlAccount", { data: { status: "invalid" } })
  ],
  ["was unlinked", () => setStore("settings", { data: { gdlAccountId: null } })]
])(
  "edit-profile whose account %s points to Settings > Accounts instead of reopening the dialog",
  async (_kind, lose) => {
    const root = mount()
    input(root, 0, "Typed for A")
    lose()
    await settle()
    expect(root.textContent).toContain("gdl_account_gone_close")
    expect(root.textContent).not.toContain("gdl_account_changed_reopen")
    expect(root.textContent).not.toContain("general:_trn_retry")
    expectLocked(root)
    expect(c.mutate).not.toHaveBeenCalled()
  }
)

it("edit-profile Accounts-card Retry is busy until its check lands, and a second click starts nothing", async () => {
  // A cached answer whose last refetch failed: the card shows the warning.
  setPeek("A", { isError: true })
  const root = mount(true)
  expect(root.textContent).toContain("gdl_account_unavailable")
  const wait = deferred()
  c.refetch.mockReturnValueOnce(wait.promise)
  const retry = button(root, "general:_trn_retry")
  retry.click()
  await settle()
  expect(retry.disabled).toBe(true)
  expect(retry.dataset.loading).toBe("true")
  retry.click()
  expect(c.refetch).toHaveBeenCalledTimes(1)
  // While the retry runs, the card says it is checking, not the warning.
  setPeek("A", { isFetching: true })
  await settle()
  expect(root.textContent).toContain("checking_gdl_account")
  expect(root.textContent).not.toContain("gdl_account_unavailable")
  setPeek("A", { isFetching: false, isError: false })
  wait.resolve(undefined)
  await settle()
  expect(root.textContent).not.toContain("checking_gdl_account")
  expect(root.textContent).not.toContain("gdl_account_unavailable")
})
