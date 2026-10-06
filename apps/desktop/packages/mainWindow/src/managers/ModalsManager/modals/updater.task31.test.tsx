import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import { createSignal } from "solid-js"
import { deferred, settle, toast } from "@/task31.fixture"
import ModsUpdater from "./ModsUpdater"
import { useAddonMutations } from "@/pages/Library/Instance/Tabs/Addons/hooks/useAddonMutations"

const c = vi.hoisted(() => ({
  accept: vi.fn(),
  read: vi.fn(),
  close: vi.fn(),
  open: vi.fn(),
  route: { id: "7" }
}))
vi.mock("@gd/ui", async () => import("@/task31.fixture"))
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => (
    <span data-key={p.key}>
      {p.key}:{JSON.stringify(p.options)}
    </span>
  )
}))
vi.mock("@solidjs/router", () => ({ useParams: () => c.route }))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ navigate: vi.fn() })
}))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ closeModal: c.close, openModal: c.open })
}))
vi.mock("@/managers/ModalsManager/ModalLayout", () => ({
  default: (p: any) => <div>{p.children}</div>
}))
vi.mock("@/utils/rspcClient", () => ({
  queryClient: {},
  rspc: {
    createQuery: () => ({ data: false }),
    createMutation: (f: any) => ({
      mutateAsync: (v: any) =>
        f().mutationKey[0] === "instance.updateMod"
          ? c.accept(v)
          : Promise.resolve()
    }),
    useContext: () => ({ client: { query: c.read } })
  }
}))

let dispose: (() => void) | undefined
beforeEach(() => {
  vi.useFakeTimers()
  vi.clearAllMocks()
  c.route.id = "7"
  c.accept.mockReset()
  c.read.mockReset()
})
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
  vi.useRealTimers()
})
const running = {
  type: "Running",
  value: { progress: { type: "Known", value: 0.25 } }
}
const failed = {
  type: "Failed",
  value: {
    progress: {
      type: "Failed",
      value: { cause: [{ display: "synthetic later failure" }] }
    }
  }
}
function mount(
  items = [{ id: "a", filename: "a.jar", has_update: true, addon_type: "mods" }]
) {
  const root = document.createElement("div")
  document.body.append(root)
  const refetch = vi.fn(async () => undefined)
  let data: any, setData: any
  dispose = render(() => {
    const hook = useAddonMutations(refetch, {} as any, vi.fn())
    void hook.handleUpdateSelected(items as any)
    data = c.open.mock.calls[0][1]
    const [value, set] = createSignal(data)
    setData = set
    return <ModsUpdater title="Synthetic updater" data={value()} />
  }, root)
  return { root, refetch, setData, data }
}
it("0036 rejected RPC is an item failure, settled count, terminal refetch and no success", async () => {
  c.accept.mockRejectedValue(new Error("synthetic acceptance rejection"))
  const { root, refetch } = mount()
  await settle()
  expect(toast.error).toHaveBeenCalledWith("Failed to update a.jar", {
    description: "synthetic acceptance rejection"
  })
  expect(root.textContent).toContain('"mods":1,"total":1')
  expect(toast.success).not.toHaveBeenCalled()
  expect(refetch).toHaveBeenCalledTimes(1)
  expect(c.close).toHaveBeenCalledTimes(1)
  expect(c.read).not.toHaveBeenCalled()
})
it("0036 mixed batch counts settled attempts and waits for final positive outcome before refetch", async () => {
  c.accept
    .mockRejectedValueOnce(new Error("first rejected"))
    .mockResolvedValueOnce(22)
  c.read
    .mockResolvedValueOnce(running)
    .mockResolvedValueOnce({ type: "Succeeded" })
  const { root, refetch } = mount([
    { id: "a", filename: "a.jar", has_update: true, addon_type: "mods" },
    { id: "b", filename: "b.jar", has_update: true, addon_type: "mods" }
  ])
  await settle()
  expect(root.textContent).toContain('"mods":1,"total":2')
  expect(root.textContent).toContain("b.jar")
  expect(refetch).not.toHaveBeenCalled()
  await vi.advanceTimersByTimeAsync(500)
  await settle()
  expect(root.textContent).toContain('"mods":2,"total":2')
  expect(toast.success).not.toHaveBeenCalled()
  expect(refetch).toHaveBeenCalledTimes(1)
  expect(c.close).toHaveBeenCalledTimes(1)
})
it("0036 long Running never completes at the old two second boundary, then later Failed is reported", async () => {
  c.accept.mockResolvedValue(11)
  let terminal = false
  c.read.mockImplementation(async () => (terminal ? failed : running))
  const { root, refetch } = mount()
  await settle()
  await vi.advanceTimersByTimeAsync(3000)
  expect(root.textContent).toContain('"mods":0,"total":1')
  expect(refetch).not.toHaveBeenCalled()
  expect(c.close).not.toHaveBeenCalled()
  expect(toast.success).not.toHaveBeenCalled()
  terminal = true
  await vi.advanceTimersByTimeAsync(500)
  await settle()
  expect(toast.error).toHaveBeenCalledWith("Failed to update a.jar", {
    description: "synthetic later failure"
  })
  expect(refetch).toHaveBeenCalledTimes(1)
  expect(toast.success).not.toHaveBeenCalled()
})
it.each([undefined, null, { type: "Unavailable" }, { type: "Dismissed" }])(
  "0036 unknown/dismissed %j never certifies success",
  async (outcome) => {
    c.accept.mockResolvedValue(11)
    c.read.mockResolvedValue(outcome)
    const { refetch } = mount()
    await settle()
    expect(toast.error).toHaveBeenCalledTimes(1)
    expect(toast.success).not.toHaveBeenCalled()
    expect(refetch).toHaveBeenCalledTimes(1)
    expect(c.close).toHaveBeenCalledTimes(1)
  }
)
it("0036 read error stays pending and recovers only after positive success", async () => {
  c.accept.mockResolvedValue(11)
  c.read
    .mockRejectedValueOnce(new Error("synthetic read failure"))
    .mockResolvedValueOnce({ type: "Succeeded" })
  const { root, refetch } = mount()
  await settle()
  expect(root.textContent).toContain('"mods":0')
  expect(refetch).not.toHaveBeenCalled()
  expect(toast.success).not.toHaveBeenCalled()
  await vi.advanceTimersByTimeAsync(500)
  await settle()
  expect(toast.success).toHaveBeenCalledTimes(1)
  expect(refetch).toHaveBeenCalledTimes(1)
})
it("0036 immediate success uses captured item/instance and originating terminal callback", async () => {
  const acceptance = deferred<number>()
  c.accept.mockReturnValueOnce(acceptance.promise).mockResolvedValueOnce(22)
  c.read.mockResolvedValue({ type: "Succeeded" })
  const { data, setData, refetch } = mount([
    { id: "a", filename: "a.jar", has_update: true, addon_type: "mods" },
    { id: "b", filename: "b.jar", has_update: true, addon_type: "mods" }
  ])
  const other = vi.fn()
  setData({
    ...data,
    instanceId: 99,
    mods: [{ id: "z", filename: "z.jar" }],
    onComplete: other
  })
  c.route.id = "99"
  acceptance.resolve(11)
  await settle()
  expect(c.accept.mock.calls.map(([v]) => v)).toEqual([
    { instance_id: 7, mod_id: "a" },
    { instance_id: 7, mod_id: "b" }
  ])
  expect(c.read.mock.calls.map(([v]) => v)).toEqual([
    ["vtask.getTaskOutcome", 11],
    ["vtask.getTaskOutcome", 22]
  ])
  expect(toast.success).toHaveBeenCalledWith(
    "Selected addons updated successfully!"
  )
  expect(refetch).toHaveBeenCalledTimes(1)
  expect(other).not.toHaveBeenCalled()
  expect(c.close).toHaveBeenCalledTimes(1)
})
