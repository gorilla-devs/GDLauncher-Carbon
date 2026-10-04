import { beforeEach, describe, expect, it, vi } from "vitest"
import type { Mod } from "@gd/core_module/bindings"

const state = vi.hoisted(() => ({
  params: { id: "1" },
  preference: { isLoading: true, data: undefined as boolean | undefined },
  ensureQueryData: vi.fn(),
  deleteMod: vi.fn(),
  openModal: vi.fn(),
  query: vi.fn(),
  updateMod: vi.fn(),
  errorToast: vi.fn()
}))

vi.mock(
  "@/components/ModDownloadButton/hooks/resolveTaskPoll",
  () =>
    import("../../../../../../components/ModDownloadButton/hooks/resolveTaskPoll")
)
vi.mock("@gd/ui", () => ({ toast: { error: state.errorToast } }))
vi.mock("@solidjs/router", () => ({ useParams: () => state.params }))
vi.mock("@/utils/rspcClient", () => ({
  queryClient: { ensureQueryData: state.ensureQueryData },
  rspc: {
    useContext: () => ({ client: { query: state.query } }),
    createQuery: () => state.preference,
    createMutation: (options: () => { mutationKey: string[] }) => ({
      mutateAsync:
        options().mutationKey[0] === "instance.deleteMod"
          ? state.deleteMod
          : state.updateMod,
      mutate: vi.fn()
    })
  }
}))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ navigate: vi.fn() })
}))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ openModal: state.openModal })
}))
vi.mock("@/pages/Library/shared/addons/addonCapabilities", () => ({
  requiresDeletionConfirmation: (type: string) => type === "worlds"
}))
vi.mock("solid-js", () => ({ onCleanup: vi.fn() }))

import { useAddonMutations } from "./useAddonMutations"

const addon = (id: string, addon_type: Mod["addon_type"] = "worlds"): Mod => ({
  id,
  filename: `name-${id}`,
  addon_type,
  enabled: true,
  metadata: null,
  curseforge: null,
  modrinth: null,
  has_update: false,
  is_duplicate: false,
  file_size: 0
})

function setup() {
  const refetch = vi.fn().mockResolvedValue(undefined)
  const optimistic = {
    optimisticToggleAddon: vi.fn(),
    optimisticDeleteAddon: vi.fn(),
    optimisticDeleteAddons: vi.fn(),
    optimisticUpdateAddon: vi.fn(),
    rollbackToServerState: vi.fn(),
    startUpdatingMod: vi.fn(),
    stopUpdatingMod: vi.fn()
  }
  const selection = vi.fn()
  return {
    actions: useAddonMutations(refetch, optimistic, selection),
    refetch,
    optimistic,
    selection
  }
}

function pendingPreference() {
  let resolve!: (value: boolean) => void
  state.ensureQueryData.mockReturnValue(
    new Promise<boolean>((done) => {
      resolve = done
    })
  )
  return resolve
}

beforeEach(() => {
  vi.clearAllMocks()
  state.params.id = "1"
  state.preference.isLoading = true
  state.preference.data = undefined
  state.deleteMod.mockResolvedValue(undefined)
})

describe("addon deletion target identity", () => {
  it("keeps the original instance while waiting for the preference", async () => {
    const resolve = pendingPreference()
    const { actions, refetch, optimistic, selection } = setup()
    const pending = actions.handleDeleteMod(addon("world-a"))
    state.params.id = "2"
    resolve(true)
    await pending

    expect(state.deleteMod).toHaveBeenCalledWith({
      instance_id: 1,
      mod_id: "world-a"
    })
    expect(refetch).not.toHaveBeenCalled()
    expect(optimistic.optimisticDeleteAddon).not.toHaveBeenCalled()
    expect(selection).not.toHaveBeenCalled()
  })

  it("keeps the instance and addon shown in a confirmation", async () => {
    state.preference.isLoading = false
    state.preference.data = false
    const { actions } = setup()
    const world = addon("world-a")
    await actions.handleDeleteMod(world)
    const confirmation = state.openModal.mock.calls[0][1]
    expect(confirmation.worldName).toBe("name-world-a")
    state.params.id = "2"
    world.id = "world-b"
    confirmation.onConfirm()

    expect(state.deleteMod).toHaveBeenCalledWith({
      instance_id: 1,
      mod_id: "world-a"
    })
  })

  it("snapshots bulk targets before waiting or changing the selection", async () => {
    const resolve = pendingPreference()
    const { actions, refetch, optimistic, selection } = setup()
    const selected = [addon("world-a"), addon("mod-a", "mods")]
    const pending = actions.handleDeleteSelected(selected)
    state.params.id = "2"
    selected[0].id = "world-b"
    selected.splice(1, 1, addon("mod-b", "mods"))
    resolve(true)
    await pending

    expect(state.deleteMod.mock.calls.map(([target]) => target)).toEqual([
      { instance_id: 1, mod_id: "world-a" },
      { instance_id: 1, mod_id: "mod-a" }
    ])
    expect(refetch).not.toHaveBeenCalled()
    expect(optimistic.optimisticDeleteAddons).not.toHaveBeenCalled()
    expect(selection).not.toHaveBeenCalled()
  })

  it("keeps bulk confirmation counts and the original target set", async () => {
    state.preference.isLoading = false
    state.preference.data = false
    const { actions } = setup()
    const selected = [addon("world-a"), addon("mod-a", "mods")]
    await actions.handleDeleteSelected(selected)
    const confirmation = state.openModal.mock.calls[0][1]
    expect(confirmation).toMatchObject({ worldCount: 1, totalCount: 2 })
    state.params.id = "2"
    selected.length = 0
    confirmation.onConfirm()

    expect(state.deleteMod.mock.calls.map(([target]) => target)).toEqual([
      { instance_id: 1, mod_id: "world-a" },
      { instance_id: 1, mod_id: "mod-a" }
    ])
  })

  it("still updates selection, optimistic state and the original list", async () => {
    state.preference.isLoading = false
    state.preference.data = true
    const { actions, refetch, optimistic, selection } = setup()
    await actions.handleDeleteMod(addon("world-a"))

    expect(state.deleteMod).toHaveBeenCalledWith({
      instance_id: 1,
      mod_id: "world-a"
    })
    expect(selection).toHaveBeenCalledOnce()
    expect(optimistic.optimisticDeleteAddon).toHaveBeenCalledWith("world-a")
    expect(refetch).toHaveBeenCalledOnce()
  })

  it.each(["success", "failure"])(
    "does not refresh or roll back another instance after deletion %s",
    async (outcome) => {
      state.preference.isLoading = false
      state.preference.data = true
      let resolve!: () => void
      let reject!: (error: Error) => void
      state.deleteMod.mockReturnValueOnce(
        new Promise<void>((done, fail) => {
          resolve = done
          reject = fail
        })
      )
      const errorLog = vi.spyOn(console, "error").mockImplementation(() => {})
      try {
        const { actions, refetch, optimistic } = setup()
        const pending = actions.handleDeleteMod(addon("world-a"))
        await Promise.resolve()
        expect(state.deleteMod).toHaveBeenCalledWith({
          instance_id: 1,
          mod_id: "world-a"
        })
        expect(optimistic.optimisticDeleteAddon).toHaveBeenCalledWith("world-a")
        state.params.id = "2"
        if (outcome === "success") resolve()
        else reject(new Error("synthetic deletion failure"))
        await pending

        expect(refetch).not.toHaveBeenCalled()
        expect(optimistic.rollbackToServerState).not.toHaveBeenCalled()
      } finally {
        errorLog.mockRestore()
      }
    }
  )
})

describe("captured row update outcomes", () => {
  it.each(["Failed", "Dismissed", "Unavailable", "Succeeded"] as const)(
    "settles the accepted row on %s without losing retained failure feedback",
    async (type) => {
      state.updateMod.mockResolvedValue(73)
      state.query.mockResolvedValue({
        type,
        value: {
          progress: {
            type: "Failed",
            value: { cause: [{ display: "Required library failed" }] }
          }
        }
      })
      const { actions, optimistic, refetch } = setup()
      await actions.handleUpdateMod(addon("mod-a", "mods"))
      expect(state.query).toHaveBeenCalledWith(["vtask.getTaskOutcome", 73])
      expect(optimistic.stopUpdatingMod).toHaveBeenCalledWith("mod-a")
      expect(refetch).toHaveBeenCalledOnce()
      if (type === "Failed" || type === "Unavailable")
        expect(state.errorToast).toHaveBeenCalledOnce()
      else expect(state.errorToast).not.toHaveBeenCalled()
    }
  )
  it("does not report an old accepted row outcome into another instance", async () => {
    state.updateMod.mockResolvedValue(73)
    state.query.mockImplementation(async () => {
      state.params.id = "2"
      return { type: "Failed" }
    })
    const { actions, optimistic, refetch } = setup()
    await actions.handleUpdateMod(addon("mod-a", "mods"))
    expect(optimistic.stopUpdatingMod).not.toHaveBeenCalled()
    expect(refetch).not.toHaveBeenCalled()
    expect(state.errorToast).not.toHaveBeenCalled()
  })
})
