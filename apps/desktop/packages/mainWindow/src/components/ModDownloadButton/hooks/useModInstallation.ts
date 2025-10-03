import { rspc } from "@/utils/rspcClient"
import { createSignal } from "solid-js"
import { FEUnifiedSearchResult } from "@gd/core_module/bindings"

interface UseModInstallationProps {
  addon: FEUnifiedSearchResult | undefined
  fileId?: number | string
}

export const useModInstallation = (props: UseModInstallationProps) => {
  const [instanceLoadingStates, setInstanceLoadingStates] = createSignal<
    Map<number, boolean>
  >(new Map())
  const [instanceTaskIds, setInstanceTaskIds] = createSignal<
    Map<number, number>
  >(new Map())

  const perInstanceInstallLatestMutation = rspc.createMutation(() => ({
    mutationKey: "instance.installLatestMod"
  }))

  const perInstanceInstallMutation = rspc.createMutation(() => ({
    mutationKey: "instance.installMod"
  }))

  const installLatestModMutation = rspc.createMutation(() => ({
    mutationKey: "instance.installLatestMod"
  }))

  const installModMutation = rspc.createMutation(() => ({
    mutationKey: "instance.installMod"
  }))

  const latestModInstallObj = () => {
    return props.addon?.platform === "curseforge"
      ? {
          Curseforge: parseInt(props.addon.id.toString(), 10)
        }
      : {
          Modrinth: props.addon!.id.toString()
        }
  }

  const modInstallObj = () => {
    return props.addon?.platform === "curseforge"
      ? {
          Curseforge: {
            project_id: parseInt(props.addon.id.toString(), 10),
            file_id: parseInt(props.fileId!.toString(), 10)
          }
        }
      : {
          Modrinth: {
            project_id: props.addon!.id.toString(),
            version_id: props.fileId!.toString()
          }
        }
  }

  const handleInstanceSelection = async (instanceId: number) => {
    if (!props.addon) return

    // Set loading state for this instance
    setInstanceLoadingStates((prev) => {
      const newMap = new Map(prev)
      newMap.set(instanceId, true)
      return newMap
    })

    try {
      let taskId: number

      if (!props.fileId) {
        taskId = await perInstanceInstallLatestMutation.mutateAsync({
          instance_id: instanceId,
          mod_source: latestModInstallObj()
        })
      } else {
        const replacesMod = null

        taskId = await perInstanceInstallMutation.mutateAsync({
          mod_source: modInstallObj(),
          instance_id: instanceId,
          install_deps: !replacesMod,
          replaces_mod: replacesMod
        })
      }

      setInstanceTaskIds((prev) => {
        const newMap = new Map(prev)
        newMap.set(instanceId, taskId)
        return newMap
      })
    } catch (_error) {
      setInstanceLoadingStates((prev) => {
        const newMap = new Map(prev)
        newMap.delete(instanceId)
        return newMap
      })
    }
  }

  const clearInstanceLoadingState = (instanceId: number) => {
    setInstanceLoadingStates((prev) => {
      const newMap = new Map(prev)
      newMap.delete(instanceId)
      return newMap
    })

    setInstanceTaskIds((prev) => {
      const newMap = new Map(prev)
      newMap.delete(instanceId)
      return newMap
    })
  }

  return {
    instanceLoadingStates,
    instanceTaskIds,
    installLatestModMutation,
    installModMutation,
    handleInstanceSelection,
    clearInstanceLoadingState,
    latestModInstallObj,
    modInstallObj
  }
}
