import { useParams } from "@solidjs/router"
import { rspc } from "@/utils/rspcClient"
import { useGDNavigate } from "@/managers/NavigationManager"
import { Mod as ModType, AddonType } from "@gd/core_module/bindings"
import { useModal } from "@/managers/ModalsManager"
import { onCleanup } from "solid-js"

export const useAddonMutations = (
  refetchAddons: () => Promise<any>,
  optimisticUpdates: {
    optimisticToggleAddon: (addonId: string, enabled: boolean) => void
    optimisticDeleteAddon: (addonId: string) => void
    optimisticDeleteAddons: (addonIds: string[]) => void
    optimisticUpdateAddon: (addonId: string) => void
    rollbackToServerState: () => void
    startUpdatingMod: (modId: string) => void
    stopUpdatingMod: (modId: string) => void
  },
  setRowSelection: (
    fn:
      | Record<string, boolean>
      | ((prev: Record<string, boolean>) => Record<string, boolean>)
  ) => void
) => {
  const params = useParams()
  const navigator = useGDNavigate()
  const modalsContext = useModal()

  // Track active polling intervals for cleanup
  const activeIntervals = new Set<number>()

  // Clean up all active intervals when component unmounts
  onCleanup(() => {
    activeIntervals.forEach((intervalId) => clearInterval(intervalId))
    activeIntervals.clear()
  })

  // Mutations
  const deleteModMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.deleteMod"]
  }))
  const disableModMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.disableMod"]
  }))
  const enableModMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.enableMod"]
  }))
  const updateModMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.updateMod"]
  }))
  const openFolderMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.openInstanceFolder"]
  }))

  // Action handlers
  const handleToggleMod = async (mod: ModType) => {
    const currentState = mod.enabled
    const newState = !currentState

    // Optimistic update
    optimisticUpdates.optimisticToggleAddon(mod.id, newState)

    try {
      if (newState) {
        await enableModMutation.mutateAsync({
          instance_id: parseInt(params.id, 10),
          mod_id: mod.id
        })
      } else {
        await disableModMutation.mutateAsync({
          instance_id: parseInt(params.id, 10),
          mod_id: mod.id
        })
      }
      // Refetch to ensure consistency with server state
      await refetchAddons()
    } catch (error) {
      console.error("Failed to toggle mod state:", error)
      // Rollback optimistic update on error
      optimisticUpdates.rollbackToServerState()
    }
  }

  const handleUpdateMod = async (mod: ModType) => {
    // Mark mod as updating and apply optimistic update
    optimisticUpdates.startUpdatingMod(mod.id)
    optimisticUpdates.optimisticUpdateAddon(mod.id)

    try {
      await updateModMutation.mutateAsync({
        instance_id: parseInt(params.id, 10),
        mod_id: mod.id
      })

      // Poll for the update to complete
      // The backend task runs async, so we need to wait for it
      let attempts = 0
      const maxAttempts = 20 // 10 seconds total

      const pollInterval = setInterval(async () => {
        attempts++
        await refetchAddons()

        if (attempts >= maxAttempts) {
          clearInterval(pollInterval)
          activeIntervals.delete(pollInterval as unknown as number)
          // If we hit max attempts, stop showing as updating
          optimisticUpdates.stopUpdatingMod(mod.id)
        }
      }, 500) as unknown as number

      // Track the interval for cleanup
      activeIntervals.add(pollInterval)

      // The stopUpdatingMod will be called automatically when the mod no longer has updates
      // This happens in the useAddonData reconciliation logic
    } catch (error) {
      console.error("Failed to update mod:", error)
      // Stop updating state and rollback on error
      optimisticUpdates.stopUpdatingMod(mod.id)
      optimisticUpdates.rollbackToServerState()
      throw error // Re-throw to handle in UI
    }
  }

  const handleDeleteMod = async (mod: ModType) => {
    // Deselect the addon being deleted
    setRowSelection((prev) => {
      const next = { ...prev }
      delete next[mod.id]
      return next
    })

    // Optimistic update
    optimisticUpdates.optimisticDeleteAddon(mod.id)

    try {
      await deleteModMutation.mutateAsync({
        instance_id: parseInt(params.id, 10),
        mod_id: mod.id
      })
      // Refetch to ensure consistency with server state
      await refetchAddons()
    } catch (error) {
      console.error("Failed to delete mod:", error)
      // Rollback optimistic update on error
      optimisticUpdates.rollbackToServerState()
    }
  }

  const handleDeleteSelected = async (selectedMods: ModType[]) => {
    // Clear selection (all selected items are being deleted)
    setRowSelection({})

    const selectedIds = selectedMods.map((mod) => mod.id)

    // Optimistic update
    optimisticUpdates.optimisticDeleteAddons(selectedIds)

    try {
      await Promise.all(
        selectedMods.map((mod) =>
          deleteModMutation.mutateAsync({
            instance_id: parseInt(params.id, 10),
            mod_id: mod.id
          })
        )
      )
      // Refetch to ensure consistency with server state
      await refetchAddons()
    } catch (error) {
      console.error("Failed to delete selected mods:", error)
      // Rollback optimistic update on error
      optimisticUpdates.rollbackToServerState()
    }
  }

  const handleOpenFolder = () => {
    openFolderMutation.mutate({
      folder: "Root",
      instance_id: parseInt(params.id, 10)
    })
  }

  const gotoSearchPage = (type?: AddonType) => {
    const searchParam = {
      mods: "mod",
      shaders: "shader",
      resourcepacks: "resource-pack",
      datapacks: "datapack",
      worlds: "world"
    }?.[type!]
    navigator.navigate(`/search/${searchParam ?? ""}?instanceId=${params.id}`)
  }

  const handleUpdateSelected = async (selectedMods: ModType[]) => {
    const modsWithUpdates = selectedMods.filter((mod) => mod.has_update)

    if (modsWithUpdates.length === 0) return

    // For batch updates, use the modal which handles progress
    modalsContext?.openModal(
      {
        name: "modsUpdater"
      },
      {
        instanceId: parseInt(params.id, 10),
        mods: modsWithUpdates,
        onComplete: () => {
          refetchAddons()
        }
      }
    )
  }

  const handleUpdateAll = async (allMods: ModType[]) => {
    const modsWithUpdates = allMods.filter((mod) => mod.has_update)

    if (modsWithUpdates.length === 0) return

    // For batch updates, use the modal which handles progress
    modalsContext?.openModal(
      {
        name: "modsUpdater"
      },
      {
        instanceId: parseInt(params.id, 10),
        mods: modsWithUpdates,
        onComplete: () => {
          refetchAddons()
        }
      }
    )
  }

  const handleSwitchVersion = (mod: ModType) => {
    const hasCurseforge = !!mod.curseforge
    const hasModrinth = !!mod.modrinth
    const modName = mod.metadata?.name || mod.filename

    // If no platforms available, do nothing
    if (!hasCurseforge && !hasModrinth) {
      return
    }

    const navigateToVersions = (
      platform: "curseforge" | "modrinth",
      projectId: string | number
    ) => {
      const url = `/addon/${projectId}/${platform}/versions?instanceId=${params.id}`
      navigator.navigate(url)
    }

    // If both platforms exist, ask user which one to use
    if (hasCurseforge && hasModrinth) {
      modalsContext?.openModal(
        {
          name: "platformSelection"
        },
        {
          modName,
          onSelectPlatform: (platform: "curseforge" | "modrinth") => {
            const projectId =
              platform === "curseforge"
                ? mod.curseforge!.project_id
                : mod.modrinth!.project_id
            navigateToVersions(platform, projectId)
          }
        }
      )
    } else {
      // Navigate directly to the only available platform
      if (hasCurseforge) {
        navigateToVersions("curseforge", mod.curseforge!.project_id)
      } else {
        navigateToVersions("modrinth", mod.modrinth!.project_id)
      }
    }
  }

  return {
    // Mutations
    deleteModMutation,
    disableModMutation,
    enableModMutation,
    updateModMutation,
    openFolderMutation,

    // Handlers
    handleToggleMod,
    handleUpdateMod,
    handleDeleteMod,
    handleDeleteSelected,
    handleOpenFolder,
    gotoSearchPage,
    handleUpdateSelected,
    handleUpdateAll,
    handleSwitchVersion
  }
}
