import { rspc } from "@/utils/rspcClient"
import { Switch, Match, createSignal, createEffect, createMemo } from "solid-js"
import { FEUnifiedSearchResult } from "@gd/core_module/bindings"
import useSearchContext from "@/components/SearchInputContext"
import { useModInstallation } from "./hooks/useModInstallation"
import { useInstanceSearch } from "./hooks/useInstanceSearch"
import { useTaskProgress } from "./hooks/useTaskProgress"
import { InstanceDropdown } from "./components/InstanceDropdown"
import { InstallButton } from "./components/InstallButton"
import { isModInstalledInInstance } from "./utils/instanceHelpers"

interface ModDownloadButtonProps {
  fileId?: number | string
  addon: FEUnifiedSearchResult | undefined
  onDropdownOpenChange?: (isOpen: boolean) => void
}

const ModDownloadButton = (props: ModDownloadButtonProps) => {
  const [taskId, setTaskId] = createSignal<number | null>(null)

  const searchContext = useSearchContext()

  const instanceLocked = () =>
    searchContext?.selectedInstance?.data?.modpack?.locked || false

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
  } = useInstanceSearch()

  const { loading, setLoading, progress, setProgress } = useTaskProgress(
    instanceTaskIds,
    clearInstanceLoadingState
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
  createEffect(async () => {
    if (taskId() !== null) {
      const task = rspc.createQuery(() => ({
        queryKey: ["vtask.getTask", taskId()]
      }))

      createEffect(() => {
        if (task?.data?.progress.type === "Known") {
          setProgress(Math.round(task?.data?.progress.value * 100))
        } else if (task?.data === null && taskId() !== null) {
          setLoading(false)
          setTaskId(null)
          setProgress(null)
        }
      })
    }
  })

  const installedMod = createMemo(() => {
    const mods = searchContext?.selectedInstanceMods?.data || []

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

    const instanceId = searchContext?.selectedInstance?.data?.id
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
  createEffect(() => {
    if (isInstalled()) {
      setLoading(false)
      setTaskId(null)
      setProgress(null)
    }
  })

  return (
    <Switch>
      <Match when={!searchContext?.selectedInstance?.data?.id}>
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
        />
      </Match>
      <Match when={searchContext?.selectedInstance?.data?.id}>
        <InstallButton
          loading={loading}
          progress={progress}
          isInstalled={isInstalled}
          instanceLocked={instanceLocked}
          fileId={props.fileId}
          installedMod={installedMod}
          onDownload={handleDownload}
        />
      </Match>
    </Switch>
  )
}

export default ModDownloadButton
