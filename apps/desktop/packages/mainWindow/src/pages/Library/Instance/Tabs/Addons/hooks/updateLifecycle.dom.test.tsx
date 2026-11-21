import { afterEach, beforeEach, expect, test, vi } from "vitest"
import { createRoot } from "solid-js"
import { createStore } from "solid-js/store"
import type { Mod } from "@gd/core_module/bindings"

const control = vi.hoisted(() => ({
  params: null as any,
  list: null as any,
  query: vi.fn(),
  update: vi.fn(),
  refetch: vi.fn(),
  error: vi.fn()
}))
vi.mock("@solidjs/router", () => ({ useParams: () => control.params }))
vi.mock("@gd/ui", () => ({ toast: { error: control.error } }))
vi.mock("@gd/i18n", () => ({ i18n: { t: (key: string) => key } }))
vi.mock("@/utils/rspcClient", () => ({
  queryClient: { ensureQueryData: vi.fn() },
  rspc: {
    useContext: () => ({ client: { query: control.query } }),
    createQuery: (options: any) =>
      options().queryKey[0] === "instance.getInstanceMods"
        ? {
            get data() {
              return control.list.rows
            },
            refetch: control.refetch
          }
        : { data: true, isLoading: false },
    createMutation: () => ({ mutateAsync: control.update, mutate: vi.fn() })
  }
}))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ navigate: vi.fn() })
}))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ openModal: vi.fn() })
}))
vi.mock("@/pages/Library/shared/addons/hooks", () => ({
  useAddonTableState: () => ({ setRowSelection: vi.fn() })
}))

import { useAddonData } from "./useAddonData"
import { useAddonMutations } from "./useAddonMutations"

const addon = (id: string): Mod => ({
  id,
  filename: `${id}.jar`,
  addon_type: "mods",
  enabled: true,
  metadata: null,
  curseforge: null,
  modrinth: null,
  has_update: true,
  is_duplicate: false,
  file_size: 1
})
const running = {
  type: "Running",
  value: { progress: { type: "Indeterminate" } }
}
let dispose: () => void
let setParams: any
let setList: any
function setup() {
  const [params, updateParams] = createStore({ id: "1" })
  const [list, updateList] = createStore({ rows: [addon("a"), addon("b")] })
  control.params = params
  control.list = list
  setParams = updateParams
  setList = updateList
  return createRoot((end) => {
    dispose = end
    const data = useAddonData()
    const actions = useAddonMutations(control.refetch, data, vi.fn())
    return { data, actions }
  })
}
beforeEach(() => {
  vi.useFakeTimers()
  vi.clearAllMocks()
  control.update.mockResolvedValue(73)
  control.query.mockResolvedValue(running)
  control.refetch.mockResolvedValue(undefined)
})
afterEach(() => {
  dispose?.()
  vi.useRealTimers()
})

test("composed row state survives more than twenty callbacks and installed-list reconciliation until positive success", async () => {
  const { data, actions } = setup()
  await actions.handleUpdateMod(addon("a"))
  setList("rows", 0, "has_update", false)
  await vi.advanceTimersByTimeAsync(12500)
  expect(control.query).toHaveBeenCalledTimes(26)
  expect(data.isModUpdating("a")).toBe(true)
  expect(data.addonsStore[0].has_update).toBe(false)
  expect(control.refetch).not.toHaveBeenCalled()
  control.query.mockResolvedValue({ type: "Succeeded" })
  await vi.advanceTimersByTimeAsync(500)
  expect(data.isModUpdating("a")).toBe(false)
  expect(control.refetch).toHaveBeenCalledOnce()
  expect(control.error).not.toHaveBeenCalled()
})

test("asynchronous failure retires only its own row, reports once and preserves another running row", async () => {
  const { data, actions } = setup()
  control.refetch.mockImplementation(async () => {
    setList("rows", [addon("a"), addon("b")])
  })
  control.update.mockResolvedValueOnce(73).mockResolvedValueOnce(74)
  await actions.handleUpdateMod(addon("a"))
  await actions.handleUpdateMod(addon("b"))
  control.query.mockImplementation(async ([, id]) =>
    id === 73
      ? {
          type: "Failed",
          value: {
            progress: {
              type: "Failed",
              value: { cause: [{ display: "Synthetic failure" }] }
            }
          }
        }
      : running
  )
  await vi.advanceTimersByTimeAsync(1000)
  expect(data.isModUpdating("a")).toBe(false)
  expect(data.isModUpdating("b")).toBe(true)
  expect(data.addonsStore.find((row) => row.id === "a")?.has_update).toBe(true)
  expect(data.addonsStore.find((row) => row.id === "b")?.has_update).toBe(false)
  expect(control.error).toHaveBeenCalledExactlyOnceWith(
    "content:_trn_addon_update_failed",
    { description: "Synthetic failure" }
  )
  expect(control.refetch).toHaveBeenCalledOnce()
})

test("undefined and failed reads retain progress and a pending query never overlaps itself", async () => {
  const { data, actions } = setup()
  control.query
    .mockResolvedValueOnce(undefined)
    .mockRejectedValueOnce(new Error("Synthetic read error"))
  await actions.handleUpdateMod(addon("a"))
  await vi.advanceTimersByTimeAsync(500)
  let finish!: (value: unknown) => void
  control.query.mockReturnValueOnce(
    new Promise((resolve) => {
      finish = resolve
    })
  )
  await vi.advanceTimersByTimeAsync(500)
  await vi.advanceTimersByTimeAsync(15000)
  expect(control.query).toHaveBeenCalledTimes(3)
  expect(data.isModUpdating("a")).toBe(true)
  expect(control.error).not.toHaveBeenCalled()
  finish(running)
  await Promise.resolve()
})

test("a synchronous rejection on row B cannot clear row A's live task progress", async () => {
  const { data, actions } = setup()
  await actions.handleUpdateMod(addon("a"))
  control.update.mockRejectedValueOnce(
    new Error("Synthetic admission rejection")
  )
  const log = vi.spyOn(console, "error").mockImplementation(() => {})
  await expect(actions.handleUpdateMod(addon("b"))).rejects.toThrow(
    "Synthetic admission rejection"
  )
  log.mockRestore()
  expect(data.isModUpdating("a")).toBe(true)
  expect(data.addonsStore.find((row) => row.id === "a")?.has_update).toBe(false)
  expect(data.isModUpdating("b")).toBe(false)
})

test("route reuse clears old local markers while a late task outcome cannot touch the new target", async () => {
  const { data, actions } = setup()
  let finish!: (value: unknown) => void
  control.query.mockReturnValueOnce(
    new Promise((resolve) => {
      finish = resolve
    })
  )
  const pending = actions.handleUpdateMod(addon("a"))
  await Promise.resolve()
  setParams("id", "2")
  setList("rows", [addon("a")])
  expect(data.isModUpdating("a")).toBe(false)
  expect(data.addonsStore[0].has_update).toBe(true)
  finish({ type: "Succeeded" })
  await pending
  expect(control.refetch).not.toHaveBeenCalled()
  expect(control.error).not.toHaveBeenCalled()
  await vi.advanceTimersByTimeAsync(1000)
  expect(control.query).toHaveBeenCalledOnce()
})

test("disposal revokes pending outcomes and scheduled polling", async () => {
  const { actions } = setup()
  await actions.handleUpdateMod(addon("a"))
  dispose()
  await vi.advanceTimersByTimeAsync(15000)
  expect(control.query).toHaveBeenCalledOnce()
  expect(control.refetch).not.toHaveBeenCalled()
})

test("route ABA revokes the first generation even when its task read settles after returning to A", async () => {
  const { data, actions } = setup()
  let finish!: (value: unknown) => void
  control.query.mockReturnValueOnce(
    new Promise((resolve) => {
      finish = resolve
    })
  )
  const old = actions.handleUpdateMod(addon("a"))
  await Promise.resolve()
  setParams("id", "2")
  setParams("id", "1")
  await actions.handleUpdateMod(addon("a"))
  finish({
    type: "Failed",
    value: {
      progress: {
        type: "Failed",
        value: { cause: [{ display: "Obsolete failure" }] }
      }
    }
  })
  await old
  expect(data.isModUpdating("a")).toBe(true)
  expect(control.refetch).not.toHaveBeenCalled()
  expect(control.error).not.toHaveBeenCalled()
})

test("disposal revokes a pending query outcome before it can refresh or report failure", async () => {
  const { actions } = setup()
  let finish!: (value: unknown) => void
  control.query.mockReturnValueOnce(
    new Promise((resolve) => {
      finish = resolve
    })
  )
  const old = actions.handleUpdateMod(addon("a"))
  await Promise.resolve()
  dispose()
  finish({ type: "Succeeded" })
  await old
  expect(control.refetch).not.toHaveBeenCalled()
  expect(control.error).not.toHaveBeenCalled()
})
