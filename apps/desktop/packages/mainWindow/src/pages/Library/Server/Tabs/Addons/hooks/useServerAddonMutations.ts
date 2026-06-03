import { useParams } from "@solidjs/router"
import { rspc } from "@/utils/rspcClient"
import { useGDNavigate } from "@/managers/NavigationManager"
import { FEServerId, ServerAddon } from "@gd/core_module/bindings"
import { RowSelectionState } from "@tanstack/solid-table"

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

  const serverId = () => parseInt(params.id ?? "0", 10)

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

  const handleDeleteMod = async (addon: ServerAddon) => {
    // Deselect the addon being deleted
    setRowSelection((prev) => {
      const next = { ...prev }
      delete next[addon.id]
      return next
    })

    // Optimistic update
    optimisticUpdates.optimisticDeleteAddon(addon.id)

    try {
      await deleteAddonMutation.mutateAsync({
        serverId: serverId(),
        addonId: addon.id
      })
      await refetchAddons()
    } catch (error) {
      console.error("Failed to delete addon:", error)
      optimisticUpdates.rollbackToServerState()
    }
  }

  const handleDeleteSelected = async (selectedAddons: ServerAddon[]) => {
    // Clear selection
    setRowSelection({})

    const selectedIds = selectedAddons.map((addon) => addon.id)

    // Optimistic update
    optimisticUpdates.optimisticDeleteAddons(selectedIds)

    try {
      await Promise.all(
        selectedAddons.map((addon) =>
          deleteAddonMutation.mutateAsync({
            serverId: serverId(),
            addonId: addon.id
          })
        )
      )
      await refetchAddons()
    } catch (error) {
      console.error("Failed to delete selected addons:", error)
      optimisticUpdates.rollbackToServerState()
    }
  }

  const handleOpenFolder = () => {
    openFolderMutation.mutate(serverId())
  }

  const gotoSearchPage = () => {
    const hasModloader = !!serverDetails.data?.modloaderType
    const target = hasModloader ? "mod" : "shader"
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
