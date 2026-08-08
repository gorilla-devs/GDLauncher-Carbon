import { rspc } from "@/utils/rspcClient"
import { createSignal } from "solid-js"
import { FEUnifiedSearchResult } from "@gd/core_module/bindings"
import { useModal } from "@/managers/ModalsManager"
import { toast } from "@gd/ui"
import { useTransContext } from "@gd/i18n"

interface UseModInstallationProps {
  addon: FEUnifiedSearchResult | undefined
  fileId?: number | string
  selectedServerId?: number
}

export const useModInstallation = (props: UseModInstallationProps) => {
  const [t] = useTransContext()
  const modalsContext = useModal()
  const ctx = rspc.useContext()
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

  // Server mutations
  const installLatestServerModMutation = rspc.createMutation(() => ({
    mutationKey: "server.installLatestServerMod"
  }))

  const installServerModMutation = rspc.createMutation(() => ({
    mutationKey: "server.installServerMod"
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

  const maybeOpenShaderWizard = async (
    instanceId: number
  ): Promise<boolean> => {
    if (props.addon?.type !== "shader") return false
    try {
      const recommendation = await ctx.client.query([
        "instance.checkShaderRequirements",
        instanceId
      ])
      if (recommendation.kind === "LoaderPresent") return false

      modalsContext?.openModal(
        { name: "shaderLoaderSetup" },
        {
          recommendation,
          instanceId,
          installLatest: !props.fileId,
          modSource: !props.fileId ? undefined : modInstallObj(),
          latestModSource: !props.fileId ? latestModInstallObj() : undefined,
          replacesMod: null,
          onComplete: (taskId: number | null) => {
            if (taskId !== null) {
              setInstanceTaskIds((prev) => {
                const newMap = new Map(prev)
                newMap.set(instanceId, taskId)
                return newMap
              })
            }
            setInstanceLoadingStates((prev) => {
              const newMap = new Map(prev)
              newMap.delete(instanceId)
              return newMap
            })
          }
        }
      )
      return true
    } catch (e) {
      console.error("Shader preflight failed", e)
      toast.error(t("notifications:_trn_shader_preflight_failed"))
      return false
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
      if (await maybeOpenShaderWizard(instanceId)) {
        return
      }

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

  // Server install handler - uses server mutations with server_id
  const handleServerInstall = async (serverId: number) => {
    if (!props.addon) return

    if (!props.fileId) {
      return await installLatestServerModMutation.mutateAsync({
        serverId,
        modSource: latestModInstallObj()
      })
    } else {
      return await installServerModMutation.mutateAsync({
        serverId,
        modSource: modInstallObj()
      })
    }
  }

  return {
    instanceLoadingStates,
    instanceTaskIds,
    installLatestModMutation,
    installModMutation,
    installLatestServerModMutation,
    installServerModMutation,
    handleInstanceSelection,
    handleServerInstall,
    clearInstanceLoadingState,
    latestModInstallObj,
    modInstallObj,
    maybeOpenShaderWizard
  }
}
