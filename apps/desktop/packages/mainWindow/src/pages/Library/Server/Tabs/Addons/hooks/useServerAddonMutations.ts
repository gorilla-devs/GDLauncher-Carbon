import { useParams } from "@solidjs/router"
import { rspc } from "@/utils/rspcClient"
import { useGDNavigate } from "@/managers/NavigationManager"
import { FEServerId, ServerAddon } from "@gd/core_module/bindings"
import { RowSelectionState } from "@tanstack/solid-table"
import { serverAllowedAddonTypes } from "@/utils/platformSearch"
import { toast } from "@gd/ui"
import { i18n } from "@gd/i18n"
import { useModal } from "@/managers/ModalsManager"
import { asksBeforeDeletingAddons } from "@/utils/addonDeletion"
import { deletionTarget } from "@/utils/deletionTarget"

const asQuestion = (addons: ServerAddon[]) =>
  addons.map((addon) => ({
    name: addon.filename,
    // Known on CurseForge or Modrinth, so it can be installed again.
    downloadable:
      addon.curseforgeProjectId !== null || addon.modrinthProjectId !== null
  }))

export const useServerAddonMutations = (
  refetchAddons: () => Promise<any>,
  optimisticUpdates: {
    optimisticToggleAddon: (addonId: string, enabled: boolean) => void
    optimisticDeleteAddon: (addonId: string) => void
    optimisticDeleteAddons: (addonIds: string[]) => void
    rollbackToServerState: () => void
  },
  setRowSelection: (
    fn: RowSelectionState | ((prev: RowSelectionState) => RowSelectionState)
  ) => void
) => {
  const params = useParams<{ id: string }>()
  const navigator = useGDNavigate()
  const modalsContext = useModal()

  const serverId = () => parseInt(params.id ?? "0", 10)

  const settings = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }))
  // Unknown while the settings load, which asks as if the Recycle Bin is off.
  const whereDeleted = () =>
    deletionTarget(settings.data?.deletionThroughRecycleBin)

  const serverDetails = rspc.createQuery(() => ({
    queryKey: ["server.getServerDetails", serverId() as unknown as FEServerId],
    enabled: serverId() > 0
  }))

  // Mutations
  const enableAddonMutation = rspc.createMutation(() => ({
    mutationKey: ["server.enableServerAddon"]
  }))

  const deleteAddonMutation = rspc.createMutation(() => ({
    mutationKey: ["server.deleteServerAddon"]
  }))
  const deleteAddonsMutation = rspc.createMutation(() => ({
    mutationKey: ["server.deleteServerAddons"]
  }))

  const openFolderMutation = rspc.createMutation(() => ({
    mutationKey: ["server.openServerFolder"]
  }))

  // Action handlers
  const handleToggleMod = async (addon: ServerAddon) => {
    const newEnabled = !addon.enabled

    // Optimistic update
    optimisticUpdates.optimisticToggleAddon(addon.id, newEnabled)

    try {
      await enableAddonMutation.mutateAsync({
        serverId: serverId(),
        addonId: addon.id,
        enabled: newEnabled
      })
      await refetchAddons()
    } catch (error) {
      console.error("Failed to toggle addon state:", error)
      optimisticUpdates.rollbackToServerState()
    }
  }

  const performDeleteMod = async (target: number, addon: ServerAddon) => {
    const current = () => serverId() === target
    if (current()) {
      // Deselect the addon being deleted
      setRowSelection((prev) => {
        const next = { ...prev }
        delete next[addon.id]
        return next
      })

      // Optimistic update
      optimisticUpdates.optimisticDeleteAddon(addon.id)
    }

    try {
      await deleteAddonMutation.mutateAsync({
        serverId: target,
        addonId: addon.id
      })
      if (current()) await refetchAddons()
    } catch (error) {
      console.error("Failed to delete addon:", error)
      if (current()) optimisticUpdates.rollbackToServerState()
    }
  }

  const handleDeleteMod = async (addon: ServerAddon) => {
    // The server and addon shown when the question was asked.
    const target = serverId()
    const snapshot = { ...addon }
    if (asksBeforeDeletingAddons(asQuestion([snapshot]), whereDeleted())) {
      modalsContext?.openModal(
        { name: "confirmAddonDeletion" },
        {
          addons: asQuestion([snapshot]),
          onConfirm: () => void performDeleteMod(target, snapshot)
        }
      )
      return
    }
    await performDeleteMod(target, snapshot)
  }

  const performDeleteSelected = async (
    target: number,
    selectedAddons: ServerAddon[]
  ) => {
    const current = () => serverId() === target
    const selectedIds = selectedAddons.map((addon) => addon.id)
    if (current()) {
      // Clear selection
      setRowSelection({})

      // Optimistic update
      optimisticUpdates.optimisticDeleteAddons(selectedIds)
    }

    try {
      // One request for the whole selection: with the Recycle Bin on, the
      // files go there together.
      const deleted = await deleteAddonsMutation.mutateAsync({
        serverId: target,
        addonIds: selectedIds
      })
      const failed = deleted.filter((addon) => addon.error)
      if (failed.length > 0) {
        toast.error(i18n.t("notifications:_trn_addons_not_deleted"), {
          description: failed[0].error ?? undefined
        })
      }
      if (current()) {
        await refetchAddons()
        // What could not be deleted is still there: show it again.
        if (failed.length > 0) optimisticUpdates.rollbackToServerState()
      }
    } catch (error) {
      console.error("Failed to delete selected addons:", error)
      if (current()) optimisticUpdates.rollbackToServerState()
    }
  }

  const handleDeleteSelected = async (selectedAddons: ServerAddon[]) => {
    const target = serverId()
    const snapshot = selectedAddons.map((addon) => ({ ...addon }))
    if (asksBeforeDeletingAddons(asQuestion(snapshot), whereDeleted())) {
      modalsContext?.openModal(
        { name: "confirmAddonDeletion" },
        {
          addons: asQuestion(snapshot),
          onConfirm: () => void performDeleteSelected(target, snapshot)
        }
      )
      return
    }
    await performDeleteSelected(target, snapshot)
  }

  const handleOpenFolder = () => {
    openFolderMutation.mutate(serverId())
  }

  const gotoSearchPage = () => {
    // Reuses serverAllowedAddonTypes() — the same rule allowedAddonTypes()
    // in utils/platformSearch.ts applies for its server branch — so the
    // default tab can't drift from what the search page itself allows:
    // ["mod", "datapack"] (modded) or ["datapack"] (vanilla). "shader" is
    // never a valid tab for a server.
    const target = serverAllowedAddonTypes(
      !!serverDetails.data?.modloaderType
    )[0]
    navigator.navigate(`/search/${target}?serverId=${params.id}`)
  }

  return {
    // Mutations
    enableAddonMutation,
    deleteAddonMutation,
    openFolderMutation,

    // Handlers
    handleToggleMod,
    handleDeleteMod,
    handleDeleteSelected,
    handleOpenFolder,
    gotoSearchPage
  }
}
