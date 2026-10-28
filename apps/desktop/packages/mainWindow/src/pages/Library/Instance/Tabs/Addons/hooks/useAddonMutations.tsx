import { toast } from "@gd/ui"
import {
  addonOutcomeText,
  resolveTaskPoll
} from "@/components/ModDownloadButton/hooks/resolveTaskPoll"
import { useParams } from "@solidjs/router"
import { queryClient, rspc } from "@/utils/rspcClient"
import { useGDNavigate } from "@/managers/NavigationManager"
import {
  Mod as ModType,
  AddonType,
  FEKeptAddonCopy
} from "@gd/core_module/bindings"
import { i18n } from "@gd/i18n"
import type { BrowsableAddonType } from "@/utils/browsableAddonType"
import { useModal } from "@/managers/ModalsManager"
import { requiresDeletionConfirmation } from "@/pages/Library/shared/addons/addonCapabilities"
import { createEffect, on, onCleanup } from "solid-js"
import { resolveBooleanPreference } from "./resolveBooleanPreference"

type AddonDeletionTarget = Pick<ModType, "id" | "filename" | "addon_type">

const snapshotDeletionTarget = (mod: ModType): AddonDeletionTarget => ({
  id: mod.id,
  filename: mod.filename,
  addon_type: mod.addon_type
})

// The folder held the addon twice (`x.jar` beside `x.jar.disabled`, listed
// as one row), and Delete removed only the copy the row showed. The row comes
// back showing the copy left, so say why instead of looking like a failed
// delete. Toasts render text, so file names are not HTML-escaped.
const announceKeptCopy = (kept: FEKeptAddonCopy | null | undefined) => {
  if (!kept) return
  toast.info(
    i18n.t("notifications:_trn_addon_deleted_one_copy", {
      deleted: kept.deleted,
      kept: kept.kept,
      interpolation: { escapeValue: false }
    })
  )
}

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
  const params = useParams<{ id: string }>()
  const navigator = useGDNavigate()
  const modalsContext = useModal()
  const ctx = rspc.useContext()
  const worldWarningDismissed = rspc.createQuery(() => ({
    queryKey: ["settings.getWorldDeletionWarningDismissed"]
  }))

  // `worldWarningDismissed.data` is `undefined` until the query resolves, so
  // reading it raw conflates "still loading" with "not dismissed" — a user
  // who dismissed the warning previously and deletes a world quickly after
  // mount (before the query settles) would still see the confirmation
  // dialog they already opted out of.
  const isWorldWarningDismissed = () =>
    resolveBooleanPreference(worldWarningDismissed, () =>
      queryClient
        .ensureQueryData({
          queryKey: ["settings.getWorldDeletionWarningDismissed"],
          queryFn: () =>
            ctx.client.query(["settings.getWorldDeletionWarningDismissed"])
        })
        // A rejected fetch must not read as "dismissed" — that would silently
        // skip the confirmation dialog and delete a world with no prompt.
        // Failing toward `false` instead means the worst case on error is an
        // extra confirmation the user already opted out of, never a silent
        // deletion.
        .catch(() => false)
    )

  // Track active polling intervals for cleanup
  const activeIntervals = new Set<number>()
  let disposed = false
  let routeGeneration = 0

  createEffect(
    on(
      () => params.id,
      () => {
        routeGeneration++
        activeIntervals.forEach((timer) => clearTimeout(timer))
        activeIntervals.clear()
      },
      { defer: true }
    )
  )

  // Clean up all active intervals when component unmounts
  onCleanup(() => {
    disposed = true
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
    const modId = mod.id
    const instanceId = parseInt(params.id, 10)
    const generation = routeGeneration
    const isCurrentUpdate = () =>
      isCurrentInstance(instanceId) && generation === routeGeneration
    // Mark mod as updating and apply optimistic update
    optimisticUpdates.startUpdatingMod(modId)
    optimisticUpdates.optimisticUpdateAddon(modId)

    try {
      const taskId = await updateModMutation.mutateAsync({
        instance_id: instanceId,
        mod_id: modId
      })
      const poll = async () => {
        if (!isCurrentUpdate()) return
        try {
          const task = await ctx.client.query(["vtask.getTaskOutcome", taskId])
          if (!isCurrentUpdate()) return
          const { action } = resolveTaskPoll(task, null, false)
          if (
            action.kind === "failed" ||
            action.kind === "completed" ||
            action.kind === "dismissed"
          ) {
            if (action.kind === "failed")
              toast.error(
                addonOutcomeText("content:_trn_addon_update_failed"),
                {
                  description: action.message
                }
              )
            optimisticUpdates.stopUpdatingMod(modId)
            await refetchAddons()
            return // Retained failures remain inspectable from the navbar.
          }
        } catch {
          /* A read failure cannot certify completion. */
        }
        if (!isCurrentUpdate()) return
        const timer = setTimeout(() => {
          activeIntervals.delete(timer as unknown as number)
          void poll()
        }, 500)
        activeIntervals.add(timer as unknown as number)
      }
      await poll()
    } catch (error) {
      console.error("Failed to update mod:", error)
      // Stop updating state and rollback on error
      if (isCurrentUpdate()) {
        optimisticUpdates.stopUpdatingMod(modId)
        optimisticUpdates.rollbackToServerState()
      }
      throw error // Re-throw to handle in UI
    }
  }

  const isCurrentInstance = (instanceId: number) =>
    !disposed && parseInt(params.id, 10) === instanceId

  const performDeleteMod = async (
    instanceId: number,
    mod: AddonDeletionTarget
  ) => {
    if (isCurrentInstance(instanceId)) {
      setRowSelection((prev) => {
        const next = { ...prev }
        delete next[mod.id]
        return next
      })
      optimisticUpdates.optimisticDeleteAddon(mod.id)
    }

    try {
      const kept = await deleteModMutation.mutateAsync({
        instance_id: instanceId,
        mod_id: mod.id
      })
      announceKeptCopy(kept)
      if (isCurrentInstance(instanceId)) await refetchAddons()
    } catch (error) {
      console.error("Failed to delete mod:", error)
      if (isCurrentInstance(instanceId)) {
        optimisticUpdates.rollbackToServerState()
      }
    }
  }

  const handleDeleteMod = async (mod: ModType) => {
    // Preferences and confirmation can outlive the current route or row.
    const instanceId = parseInt(params.id, 10)
    const target = snapshotDeletionTarget(mod)
    if (
      requiresDeletionConfirmation(target.addon_type) &&
      !(await isWorldWarningDismissed())
    ) {
      modalsContext?.openModal(
        { name: "confirmWorldDeletion" },
        {
          worldName: target.filename,
          onConfirm: () => void performDeleteMod(instanceId, target)
        }
      )
      return
    }

    await performDeleteMod(instanceId, target)
  }

  const performDeleteSelected = async (
    instanceId: number,
    selectedMods: AddonDeletionTarget[]
  ) => {
    const selectedIds = selectedMods.map((mod) => mod.id)
    if (isCurrentInstance(instanceId)) {
      setRowSelection({})
      optimisticUpdates.optimisticDeleteAddons(selectedIds)
    }

    try {
      const kept = await Promise.all(
        selectedMods.map((mod) =>
          deleteModMutation.mutateAsync({
            instance_id: instanceId,
            mod_id: mod.id
          })
        )
      )
      kept.forEach(announceKeptCopy)
      if (isCurrentInstance(instanceId)) await refetchAddons()
    } catch (error) {
      console.error("Failed to delete selected mods:", error)
      if (isCurrentInstance(instanceId)) {
        optimisticUpdates.rollbackToServerState()
      }
    }
  }

  const handleDeleteSelected = async (selectedMods: ModType[]) => {
    const instanceId = parseInt(params.id, 10)
    const targets = selectedMods.map(snapshotDeletionTarget)
    const worldsSelected = targets.filter((mod) =>
      requiresDeletionConfirmation(mod.addon_type)
    )

    if (worldsSelected.length > 0 && !(await isWorldWarningDismissed())) {
      modalsContext?.openModal(
        { name: "confirmWorldDeletion" },
        {
          worldName: worldsSelected[0].filename,
          worldCount: worldsSelected.length,
          // Confirming deletes the whole selection, not only the worlds in
          // it — the dialog is raised *because* a world is among them, but
          // `performDeleteSelected` below removes every selected addon. The
          // dialog names that total so it describes what the button does.
          totalCount: targets.length,
          onConfirm: () => void performDeleteSelected(instanceId, targets)
        }
      )
      return
    }

    await performDeleteSelected(instanceId, targets)
  }

  const handleOpenFolder = () => {
    openFolderMutation.mutate({
      folder: "Root",
      instance_id: parseInt(params.id, 10)
    })
  }

  const gotoSearchPage = (type?: AddonType) => {
    const searchTypes: Record<AddonType, BrowsableAddonType> = {
      mods: "mod",
      shaders: "shader",
      resourcepacks: "resourcePack",
      datapacks: "datapack",
      worlds: "world"
    }
    const searchParam = type ? searchTypes[type] : undefined
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
        onComplete: () => refetchAddons()
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
        onComplete: () => refetchAddons()
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
