import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { createRoot } from "solid-js"
import { createStore } from "solid-js/store"
import { useSavedGdlOwner } from "./savedGdlOwner"

const c = vi.hoisted(() => ({
  store: undefined as any,
  peek: undefined as any,
  refetch: vi.fn()
}))
vi.mock("@/components/GlobalStoreContext", () => ({
  useGlobalStore: () => c.store
}))
vi.mock("@/utils/rspcClient", () => ({
  queryClient: {
    getQueryData: ([_key, id]: any) => c.peek[id]?.data
  },
  rspc: {
    createQuery: (f: any) => ({
      get data() {
        return c.peek[f().queryKey[1]]?.data
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

const user = (id = "A", over: Record<string, unknown> = {}) => ({
  microsoftOid: `oid${id}`,
  friendCode: `friend${id}`,
  displayName: `Owner${id}`,
  email: `${id}@synthetic.invalid`,
  ...over
})
let setStore: any, setPeek: any, dispose: (() => void) | undefined
beforeEach(() => {
  c.refetch.mockReset()
  c.refetch.mockResolvedValue(undefined)
  const [store, set] = createStore({
    settings: { data: { gdlAccountId: "A" } },
    gdlAccount: { data: { status: "valid", value: user() } }
  })
  c.store = store
  setStore = set
  const [peek, setP] = createStore<any>({
    A: { data: user(), isPending: false, isFetching: false, isError: false },
    B: { data: user("B"), isPending: false, isFetching: false, isError: false }
  })
  c.peek = peek
  setPeek = setP
})
afterEach(() => {
  dispose?.()
  dispose = undefined
})
function start() {
  return createRoot((d) => {
    dispose = d
    return useSavedGdlOwner()
  })
}
// solid-query replaces `data` wholesale on every refetch.
const refetchProfile = (value: Record<string, unknown>) =>
  setStore("gdlAccount", { data: { status: "valid", value } })

it("wholesale refetches of the same identity leave the generation and owner alone", () => {
  const hook = start()
  const captured = hook.capture()!
  expect(captured).toBeDefined()
  const before = hook.generation()
  refetchProfile({ ...user(), displayName: "Renamed", isEmailVerified: true })
  setStore("settings", { data: { gdlAccountId: "A" } })
  setPeek("A", { data: user() })
  refetchProfile(user())
  expect(hook.generation()).toBe(before)
  expect(hook.current(captured)).toBe(true)
  expect(hook.status()).toBe("owned")
})

it.each([
  ["uuid", () => setStore("settings", { data: { gdlAccountId: "B" } })],
  [
    "microsoftOid",
    () => refetchProfile({ ...user(), microsoftOid: "otherOid" })
  ],
  ["friendCode", () => refetchProfile({ ...user(), friendCode: "otherFriend" })]
])("changing %s bumps the generation exactly once", (_field, change) => {
  const hook = start()
  const owner = hook.capture()!
  const before = hook.generation()
  change()
  expect(hook.generation()).toBe(before + 1)
  // Refetching the new identity again is not another change.
  change()
  expect(hook.generation()).toBe(before + 1)
  expect(hook.current(owner)).toBe(false)
})

it("the first keyed answer only records it, so an owner captured from it stays current", () => {
  setPeek("A", { data: undefined, isPending: true, isFetching: true })
  const hook = start()
  expect(hook.status()).toBe("loading")
  expect(hook.capture()).toBeUndefined()
  const before = hook.generation()
  // query-core writes the flags before data in one merge.
  setPeek("A", { isPending: false, isFetching: false, data: user() })
  expect(hook.generation()).toBe(before)
  const owner = hook.capture()
  expect(owner?.generation).toBe(before)
  expect(hook.current(owner)).toBe(true)
  expect(hook.status()).toBe("owned")
})

it.each([
  ["another account", () => user("B")],
  ["null", () => null],
  ["missing", () => undefined]
])(
  "a known keyed answer that turns into %s retires the owner, and so does its return",
  (_kind, next) => {
    const hook = start()
    const owner = hook.capture()!
    const before = hook.generation()
    setPeek("A", "data", next())
    expect(hook.generation()).toBe(before + 1)
    expect(hook.current(owner)).toBe(false)
    expect(hook.status()).toBe("unavailable")
    setPeek("A", "data", user())
    expect(hook.generation()).toBe(before + 2)
    expect(hook.current(owner)).toBe(false)
    // The same account can be certified again, at the new generation.
    expect(hook.capture()?.generation).toBe(before + 2)
  }
)

it("a cached keyed answer certifies the owner while a background refetch runs", () => {
  setPeek("A", "isFetching", true)
  const hook = start()
  expect(hook.status()).toBe("owned")
  expect(hook.capture()?.uuid).toBe("A")
})

it("an errored or mismatched keyed answer is unavailable, not loading", () => {
  setPeek("A", { data: undefined, isPending: false, isError: true })
  expect(start().status()).toBe("unavailable")
  dispose?.()
  setPeek("A", { data: user("B"), isError: false })
  expect(start().status()).toBe("unavailable")
})

it("a check running after a failed one is loading, not a fresh failure", () => {
  setPeek("A", { isError: true, isFetching: false })
  const hook = start()
  expect(hook.status()).toBe("unavailable")
  setPeek("A", { isFetching: true })
  expect(hook.status()).toBe("loading")
  setPeek("A", { isFetching: false })
  expect(hook.status()).toBe("unavailable")
  setPeek("A", { isError: false })
  expect(hook.status()).toBe("owned")
})

it("refetch asks for the saved UUID only when there is something to check", async () => {
  const hook = start()
  await hook.refetch()
  expect(c.refetch).toHaveBeenCalledWith("A")
  c.refetch.mockClear()
  setStore("settings", { data: { gdlAccountId: null } })
  await hook.refetch()
  expect(c.refetch).not.toHaveBeenCalled()
})
