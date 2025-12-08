import { rspc } from "@/utils/rspcClient"
import { Switch, Match, createSignal, createEffect, createMemo } from "solid-js"
import { FEUnifiedSearchResult, Mod } from "@gd/core_module/bindings"
import { useModInstallation } from "./hooks/useModInstallation"
import { useInstanceSearch } from "./hooks/useInstanceSearch"
import { useTaskProgress } from "./hooks/useTaskProgress"
import { InstanceDropdown } from "./components/InstanceDropdown"
import { InstallButton } from "./components/InstallButton"
import { toast } from "@gd/ui"

interface ModDownloadButtonProps {
  fileId?: number | string
  addon: FEUnifiedSearchResult | undefined
  onDropdownOpenChange?: (isOpen: boolean) => void
  selectedInstanceId?: number
  selectedInstanceMods?: Mod[]
  instanceLocked?: boolean
  size?: "small" | "medium" | "large"
  iconOnly?: boolean
}

const ModDownloadButton = (props: ModDownloadButtonProps) => {
  const [taskId, setTaskId] = createSignal<number | null>(null)

  const {
    instanceLoadingStates,
    instanceTaskIds,
    installLatestModMutation,
    installModMutation,
    handleInstanceSelection,
    clearInstanceLoadingState,
    latestModInstallObj,
    modInstallObj
  } = useModInstallation(props)

  const {
    searchQuery,
    setSearchQuery,
    hoveredInstanceId,
    setHoveredInstanceId,
    filteredInstances,
    shouldVirtualize
  } = useInstanceSearch({ addonType: props.addon?.type })

  const { loading, setLoading, progress, setProgress } = useTaskProgress(
    instanceTaskIds,
    clearInstanceLoadingState,
    props.addon
  )

  createEffect(() => {
    if (installLatestModMutation.isPending) {
      setLoading(true)
    }

    if (installLatestModMutation.isSuccess) {
      setTaskId(installLatestModMutation.data)
    }
  })

  createEffect(() => {
    if (installModMutation.isPending) {
      setLoading(true)
    }

    if (installModMutation.isSuccess) {
      setTaskId(installModMutation.data)
    }
  })

  // Handle task progress for single instance button
  const task = rspc.createQuery(() => ({
    queryKey: ["vtask.getTask", taskId()],
    enabled: taskId() !== null
  }))

  createEffect(() => {
    if (taskId() !== null) {
      if (task?.data?.progress.type === "Known") {
        setProgress(Math.round(task?.data?.progress.value * 100))
      } else if (task?.data === null) {
        setLoading(false)
        setTaskId(null)
        setProgress(null)
      }
    }
  })

  const installedMod = createMemo(() => {
    const mods = props.selectedInstanceMods || []

    const found = mods.find((mod) => {
      if (!props.addon) return false

      if (props.addon.platform === "curseforge") {
        return (
          mod.curseforge?.project_id === parseInt(props.addon.id.toString(), 10)
        )
      } else if (props.addon.platform === "modrinth") {
        return mod.modrinth?.project_id === props.addon.id.toString()
      }

      return false
    })

    return found
  })

  const isInstalled = createMemo(() => {
    const localInstalledMod = installedMod()

    if (!localInstalledMod || !props.addon) return false

    if (!props.fileId) {
      // Installing latest version - just check if mod exists
      return !!localInstalledMod
    } else {
      // Installing specific version - check if exact version matches
      if (props.addon.platform === "curseforge") {
        return (
          localInstalledMod.curseforge!.file_id ===
          parseInt(props.fileId.toString(), 10)
        )
      } else {
        return (
          localInstalledMod.modrinth!.version_id === props.fileId.toString()
        )
      }
    }
  })

  const handleDownload = async () => {
    if (!props.addon) return

    const instanceId = props.selectedInstanceId
    if (!instanceId || isInstalled()) return

    if (!props.fileId) {
      await installLatestModMutation.mutateAsync({
        instance_id: instanceId,
        mod_source: latestModInstallObj()
      })
    } else {
      const replacesMod = installedMod()?.id || null

      await installModMutation.mutateAsync({
        mod_source: modInstallObj(),
        instance_id: instanceId,
        install_deps: !replacesMod,
        replaces_mod: replacesMod
      })
    }
  }

  // Watch for installation completion and clear states reactively
  const [wasInstalled, setWasInstalled] = createSignal(false)
  const [wasLoading, setWasLoading] = createSignal(false)
  const [isInitialized, setIsInitialized] = createSignal(false)

  createEffect(() => {
    const installed = isInstalled()
    const isCurrentlyLoading = loading()
    const isWorld = props.addon?.type === "world"

    // For worlds: show toast when loading finishes (since they never show as "installed")
    if (isWorld && wasLoading() && !isCurrentlyLoading && taskId() === null) {
      toast.success(`${props.addon?.title || "World"} installed successfully`, {
        duration: 2000
      })
    }

    // For other addon types: show toast when transitioning from not installed to installed
    // Skip on initial mount to avoid showing toast for already-installed versions
    if (!isWorld && installed && !wasInstalled() && isInitialized()) {
      toast.success(`${props.addon?.title || "Addon"} installed successfully`, {
        duration: 2000
      })
    }

    // Track installed state changes
    if (installed !== wasInstalled()) {
      setWasInstalled(installed)
    }

    // Track loading state changes
    setWasLoading(isCurrentlyLoading)

    if (installed || (isWorld && taskId() === null)) {
      setLoading(false)
      setTaskId(null)
      setProgress(null)
    }

    // Mark as initialized after first run
    if (!isInitialized()) {
      setIsInitialized(true)
    }
  })

  return (
    <Switch>
      <Match when={!props.selectedInstanceId}>
        <InstanceDropdown
          addon={props.addon}
          filteredInstances={filteredInstances}
          shouldVirtualize={shouldVirtualize}
          searchQuery={searchQuery}
          setSearchQuery={setSearchQuery}
          hoveredInstanceId={hoveredInstanceId}
          setHoveredInstanceId={setHoveredInstanceId}
          instanceLoadingStates={instanceLoadingStates}
          clearInstanceLoadingState={clearInstanceLoadingState}
          handleInstanceSelection={handleInstanceSelection}
          onDropdownOpenChange={props.onDropdownOpenChange}
          size={props.size}
          iconOnly={props.iconOnly}
        />
      </Match>
      <Match when={props.selectedInstanceId}>
        <InstallButton
          loading={loading}
          progress={progress}
          isInstalled={isInstalled}
          instanceLocked={() => props.instanceLocked ?? false}
          fileId={props.fileId}
          installedMod={installedMod}
          onDownload={handleDownload}
          size={props.size}
          iconOnly={props.iconOnly}
        />
      </Match>
    </Switch>
  )
}

export default ModDownloadButton
