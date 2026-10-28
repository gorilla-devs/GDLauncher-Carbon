import { rspc } from "@/utils/rspcClient"
import { createSignal, onCleanup } from "solid-js"
import { FEUnifiedSearchResult } from "@gd/core_module/bindings"
import { useModal } from "@/managers/ModalsManager"
import { toast } from "@gd/ui"
import { useTransContext } from "@gd/i18n"
import {
  createLatestModInstallObj,
  createModInstallObj
} from "../utils/modInstallObjects"
import { effectiveFileId, LinkedFile, LinkTarget } from "@/utils/linkedFile"

interface UseModInstallationProps {
  addon: FEUnifiedSearchResult | undefined
  fileId?: number | string
  linkedFile?: LinkedFile
  selectedServerId?: number
}

interface UseModInstallationOptions {
  /** Release and loader of an instance, for checking a linked file. */
  instanceTarget?: (instanceId: number) => LinkTarget | undefined
  /** Release and loader of a server, for checking a linked file. */
  serverTarget?: (serverId: number) => LinkTarget | undefined
}

export interface InstanceInstallTask {
  taskId: number
  title: string
  type: FEUnifiedSearchResult["type"]
}

export const useModInstallation = (
  props: UseModInstallationProps,
  options: UseModInstallationOptions = {}
) => {
  const [t] = useTransContext()
  const modalsContext = useModal()
  const ctx = rspc.useContext()
  let active = true
  const attempts = new Map<number, symbol>()
  onCleanup(() => {
    active = false
    attempts.clear()
  })
  const [instanceLoadingStates, setInstanceLoadingStates] = createSignal<
    Map<number, boolean>
  >(new Map())
  const [instanceTasks, setInstanceTasks] = createSignal<
    Map<number, InstanceInstallTask>
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

  // Build both wire variants before requirements yields to reactive selection.
  const captureInstallAttempt = (
    instanceId: number,
    replacesMod: string | null = null
  ) => {
    const addon = props.addon
    if (!addon) return undefined
    // A link's file that does not fit this instance installs the instance's
    // own version instead (2.0.40's path), and never replaces an installed copy.
    const fileId = effectiveFileId(
      props.fileId,
      props.linkedFile,
      addon.type,
      options.instanceTarget?.(instanceId)
    )
    const owner = Symbol()
    attempts.set(instanceId, owner)
    return {
      owner,
      instanceId,
      fileId,
      addonType: addon.type,
      title: addon.title,
      installLatest: !fileId,
      modSource: fileId ? createModInstallObj(addon, fileId) : undefined,
      latestModSource: !fileId ? createLatestModInstallObj(addon) : undefined,
      replacesMod: fileId ? replacesMod : null
    }
  }
  type InstallAttempt = NonNullable<ReturnType<typeof captureInstallAttempt>>
  const current = (attempt: InstallAttempt) =>
    active && attempts.get(attempt.instanceId) === attempt.owner

  const maybeOpenShaderWizard = async (
    attempt: InstallAttempt
  ): Promise<"direct" | "wizard" | "retired"> => {
    if (attempt.addonType !== "shader") return "direct"
    const instanceId = attempt.instanceId
    try {
      const recommendation = await ctx.client.query([
        "instance.checkShaderRequirements",
        instanceId
      ])
      if (active && !current(attempt)) return "retired"
      // A captured direct install can continue in the background after disposal.
      // A new wizard requires a live producer; retirement never means fallback.
      if (recommendation.kind === "LoaderPresent") return "direct"
      if (!current(attempt)) return "retired"

      modalsContext?.openModal(
        { name: "shaderLoaderSetup" },
        {
          recommendation,
          instanceId,
          installLatest: attempt.installLatest,
          modSource: attempt.modSource,
          latestModSource: attempt.latestModSource,
          replacesMod: attempt.replacesMod,
          onComplete: (taskId: number | null) => {
            if (!current(attempt)) return
            if (taskId !== null) {
              setInstanceTasks((prev) => {
                const newMap = new Map(prev)
                newMap.set(instanceId, {
                  taskId,
                  title: attempt.title,
                  type: attempt.addonType
                })
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
      return "wizard"
    } catch (e) {
      if (active && !current(attempt)) return "retired"
      if (current(attempt)) {
        console.error("Shader preflight failed", e)
        toast.error(t("notifications:_trn_shader_preflight_failed"))
      }
      return "direct"
    }
  }

  const handleInstanceSelection = async (instanceId: number) => {
    const attempt = captureInstallAttempt(instanceId)
    if (!attempt) return

    // Set loading state for this instance
    setInstanceLoadingStates((prev) => {
      const newMap = new Map(prev)
      newMap.set(instanceId, true)
      return newMap
    })

    try {
      const decision = await maybeOpenShaderWizard(attempt)
      if (decision !== "direct") {
        if (decision === "retired" && current(attempt))
          clearInstanceLoadingState(instanceId)
        return
      }

      let taskId: number

      if (attempt.installLatest) {
        taskId = await perInstanceInstallLatestMutation.mutateAsync({
          instance_id: instanceId,
          mod_source: attempt.latestModSource!
        })
      } else {
        const replacesMod = attempt.replacesMod

        taskId = await perInstanceInstallMutation.mutateAsync({
          mod_source: attempt.modSource!,
          instance_id: instanceId,
          install_deps: !replacesMod,
          replaces_mod: replacesMod
        })
      }

      if (!current(attempt)) return
      setInstanceTasks((prev) => {
        const newMap = new Map(prev)
        newMap.set(instanceId, {
          taskId,
          title: attempt.title,
          type: attempt.addonType
        })
        return newMap
      })
    } catch (_error) {
      if (!current(attempt)) return
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

    setInstanceTasks((prev) => {
      const newMap = new Map(prev)
      newMap.delete(instanceId)
      return newMap
    })
  }

  // Server install handler - uses server mutations with server_id. A link's
  // file that does not fit the server installs the server's own version
  // instead (2.0.40's path), the same rule as instances.
  const handleServerInstall = async (serverId: number) => {
    const addon = props.addon
    if (!addon) return
    const fileId = effectiveFileId(
      props.fileId,
      props.linkedFile,
      addon.type,
      options.serverTarget?.(serverId)
    )

    if (!fileId) {
      return await installLatestServerModMutation.mutateAsync({
        serverId,
        modSource: createLatestModInstallObj(addon)
      })
    } else {
      return await installServerModMutation.mutateAsync({
        serverId,
        modSource: createModInstallObj(addon, fileId)
      })
    }
  }

  return {
    instanceLoadingStates,
    instanceTasks,
    installLatestModMutation,
    installModMutation,
    installLatestServerModMutation,
    installServerModMutation,
    handleInstanceSelection,
    handleServerInstall,
    clearInstanceLoadingState,
    captureInstallAttempt,
    maybeOpenShaderWizard
  }
}
