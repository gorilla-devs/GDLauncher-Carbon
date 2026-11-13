import { beforeEach, describe, expect, it, vi } from "vitest"
import type { Mod } from "@gd/core_module/bindings"

const state = vi.hoisted(() => ({
  params: { id: "1" },
  preference: { isLoading: true, data: undefined as boolean | undefined },
  settings: {
    data: { deletionThroughRecycleBin: true } as
      | { deletionThroughRecycleBin: boolean }
      | undefined
  },
  ensureQueryData: vi.fn(),
  deleteMod: vi.fn(),
  deleteMods: vi.fn(),
  openModal: vi.fn(),
  query: vi.fn(),
  updateMod: vi.fn(),
  errorToast: vi.fn(),
  infoToast: vi.fn(),
  translate: vi.fn(
    (key: string, params?: Record<string, unknown>) =>
      `${key} ${JSON.stringify(params ?? {})}`
  )
}))

vi.mock(
  "@/components/ModDownloadButton/hooks/resolveTaskPoll",
  () =>
    import("../../../../../../components/ModDownloadButton/hooks/resolveTaskPoll")
)
vi.mock(
  "@/utils/addonDeletion",
  () => import("../../../../../../utils/addonDeletion")
)
vi.mock(
  "@/utils/deletionTarget",
  () => import("../../../../../../utils/deletionTarget")
)
vi.mock("@gd/ui", () => ({
  toast: { error: state.errorToast, info: state.infoToast }
}))
vi.mock("@gd/i18n", () => ({ i18n: { t: state.translate } }))
vi.mock("@solidjs/router", () => ({ useParams: () => state.params }))
vi.mock("@/utils/rspcClient", () => ({
  queryClient: { ensureQueryData: state.ensureQueryData },
  rspc: {
    useContext: () => ({ client: { query: state.query } }),
    createQuery: (options: () => { queryKey: string[] }) =>
      options().queryKey[0] === "settings.getSettings"
        ? state.settings
        : state.preference,
    createMutation: (options: () => { mutationKey: string[] }) => ({
      mutateAsync:
        options().mutationKey[0] === "instance.deleteMod"
          ? state.deleteMod
          : options().mutationKey[0] === "instance.deleteMods"
            ? state.deleteMods
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
vi.mock("solid-js", async (importOriginal) => ({
  ...(await importOriginal<typeof import("solid-js")>()),
  onCleanup: vi.fn()
}))

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
  state.settings.data = { deletionThroughRecycleBin: true }
  state.deleteMod.mockResolvedValue(undefined)
  state.deleteMods.mockImplementation(async (request: { mod_ids: string[] }) =>
    request.mod_ids.map((mod_id) => ({ mod_id, kept: null, error: null }))
  )
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
    // The world warning is off, so the selection asks its own question.
    const [modal, question] = state.openModal.mock.calls[0]
    expect(modal).toEqual({ name: "confirmAddonDeletion" })
    expect(question.addons.map((a: { name: string }) => a.name)).toEqual([
      "name-world-a",
      "name-mod-a"
    ])
    await question.onConfirm()
    await Promise.resolve()

    expect(state.deleteMods.mock.calls.map(([target]) => target)).toEqual([
      { instance_id: 1, mod_ids: ["world-a", "mod-a"] }
    ])
    expect(state.deleteMod).not.toHaveBeenCalled()
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

    // One request for the whole selection, asked once by the world dialog.
    expect(state.openModal).toHaveBeenCalledOnce()
    expect(state.deleteMods.mock.calls.map(([target]) => target)).toEqual([
      { instance_id: 1, mod_ids: ["world-a", "mod-a"] }
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

describe("deleting one of two copies of an addon", () => {
  beforeEach(() => {
    state.preference.isLoading = false
    state.preference.data = true
  })

  it("says which copy is still in the folder", async () => {
    state.deleteMod.mockResolvedValueOnce({
      deleted: "twin.jar",
      kept: "twin.jar.disabled"
    })
    const { actions, refetch } = setup()
    await actions.handleDeleteMod(addon("twin", "mods"))

    expect(state.translate).toHaveBeenCalledWith(
      "notifications:_trn_addon_deleted_one_copy",
      {
        deleted: "twin.jar",
        kept: "twin.jar.disabled",
        interpolation: { escapeValue: false }
      }
    )
    expect(state.infoToast).toHaveBeenCalledOnce()
    expect(refetch).toHaveBeenCalledOnce()
  })

  it("stays quiet for a plain delete and names each copy left in a bulk delete", async () => {
    const { actions } = setup()
    await actions.handleDeleteMod(addon("plain", "mods"))
    expect(state.infoToast).not.toHaveBeenCalled()

    state.deleteMods.mockResolvedValueOnce([
      { mod_id: "a", kept: null, error: null },
      {
        mod_id: "b",
        kept: { deleted: "b.jar.disabled", kept: "b.jar" },
        error: null
      }
    ])
    await actions.handleDeleteSelected([addon("a", "mods"), addon("b", "mods")])
    await state.openModal.mock.calls[0][1].onConfirm()
    await vi.waitFor(() => expect(state.infoToast).toHaveBeenCalledOnce())
    expect(state.translate).toHaveBeenCalledWith(
      "notifications:_trn_addon_deleted_one_copy",
      expect.objectContaining({ deleted: "b.jar.disabled", kept: "b.jar" })
    )
  })
})

describe("asking before addons are deleted", () => {
  const fromPlatform = (id: string): Mod => ({
    ...addon(id, "mods"),
    modrinth: {} as Mod["modrinth"]
  })

  it("deletes one mod from a platform without asking, Recycle Bin on or off", async () => {
    for (const deletionThroughRecycleBin of [true, false]) {
      state.settings.data = { deletionThroughRecycleBin }
      const { actions } = setup()
      await actions.handleDeleteMod(fromPlatform("sodium"))
    }
    expect(state.openModal).not.toHaveBeenCalled()
    expect(state.deleteMod).toHaveBeenCalledTimes(2)
  })

  it("deletes one file of the user's own without asking while it goes to the Recycle Bin", async () => {
    const { actions } = setup()
    await actions.handleDeleteMod(addon("own", "mods"))
    expect(state.openModal).not.toHaveBeenCalled()
    expect(state.deleteMod).toHaveBeenCalledWith({
      instance_id: 1,
      mod_id: "own"
    })
  })

  it.each([
    ["off", { deletionThroughRecycleBin: false }],
    ["not known yet", undefined]
  ])(
    "asks for one file of the user's own with the Recycle Bin %s",
    async (_label, settings) => {
      state.settings.data = settings
      const { actions } = setup()
      const own = addon("own", "resourcepacks")
      await actions.handleDeleteMod(own)

      expect(state.deleteMod).not.toHaveBeenCalled()
      const [modal, question] = state.openModal.mock.calls[0]
      expect(modal).toEqual({ name: "confirmAddonDeletion" })
      expect(question.addons).toEqual([
        { name: "name-own", downloadable: false }
      ])
      // The addon and instance shown in the question are the ones deleted.
      state.params.id = "2"
      own.id = "other"
      question.onConfirm()
      expect(state.deleteMod).toHaveBeenCalledWith({
        instance_id: 1,
        mod_id: "own"
      })
    }
  )

  it("asks once for a selection and says which files cannot be downloaded again", async () => {
    const { actions } = setup()
    await actions.handleDeleteSelected([
      fromPlatform("sodium"),
      addon("own", "mods")
    ])

    expect(state.deleteMods).not.toHaveBeenCalled()
    expect(state.openModal).toHaveBeenCalledOnce()
    const [modal, question] = state.openModal.mock.calls[0]
    expect(modal).toEqual({ name: "confirmAddonDeletion" })
    expect(question.addons).toEqual([
      { name: "name-sodium", downloadable: true },
      { name: "name-own", downloadable: false }
    ])
    await question.onConfirm()
    expect(state.deleteMods).toHaveBeenCalledWith({
      instance_id: 1,
      mod_ids: ["sodium", "own"]
    })
  })

  it("does not ask twice for a world the user chose not to be asked about", async () => {
    state.preference.isLoading = false
    state.preference.data = true
    state.settings.data = { deletionThroughRecycleBin: false }
    const { actions } = setup()
    await actions.handleDeleteMod(addon("world-a"))
    await actions.handleDeleteSelected([addon("world-b")])
    expect(state.openModal).not.toHaveBeenCalled()
    expect(state.deleteMod).toHaveBeenCalledOnce()
    expect(state.deleteMods).toHaveBeenCalledOnce()
  })

  it("says so when part of a selection could not be deleted and shows those rows again", async () => {
    state.deleteMods.mockResolvedValueOnce([
      { mod_id: "a", kept: null, error: null },
      { mod_id: "b", kept: null, error: "Nothing was deleted, because …" }
    ])
    const { actions, refetch, optimistic } = setup()
    await actions.handleDeleteSelected([addon("a", "mods"), addon("b", "mods")])
    await state.openModal.mock.calls[0][1].onConfirm()

    await vi.waitFor(() => expect(state.errorToast).toHaveBeenCalledOnce())
    expect(state.errorToast.mock.calls[0][1]).toEqual({
      description: "Nothing was deleted, because …"
    })
    await vi.waitFor(() =>
      expect(optimistic.rollbackToServerState).toHaveBeenCalledOnce()
    )
    expect(refetch).toHaveBeenCalledOnce()
  })
})
