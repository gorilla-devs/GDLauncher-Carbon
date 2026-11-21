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
import { extractErrorDisplay } from "@/utils/rspcErrorText"
import { addonInstallFailure } from "../addonInstallScope"
import {
  instanceInstallIsDatapack,
  newestDatapackBuild,
  ReadableWorld,
  worldDatapackInstall
} from "@/utils/worldTarget"

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
  /** A data pack that lands in the instance, to be added to a world there. */
  staged: boolean
  /** Set when the data pack went straight into this world. */
  world?: string
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

  const installWorldDatapackMutation = rspc.createMutation(() => ({
    mutationKey: "instance.installWorldDatapack"
  }))

  // Server mutations
  const installLatestServerModMutation = rspc.createMutation(() => ({
    mutationKey: "server.installLatestServerMod"
  }))

  const installServerModMutation = rspc.createMutation(() => ({
    mutationKey: "server.installServerMod"
  }))

  /** Whether an install into the instance itself is the project's data pack
   *  build. Until the instance's loader is known it counts as having one, so
   *  a project that also ships as a mod keeps installing its mod. */
  const installsAsDatapack = (instanceId: number | undefined) => {
    const target =
      instanceId === undefined
        ? undefined
        : options.instanceTarget?.(instanceId)
    return instanceInstallIsDatapack(props.addon, !target || !!target.modloader)
  }

  // Build both wire variants before requirements yields to reactive selection.
  const captureInstallAttempt = (
    instanceId: number,
    replacesMod: string | null = null
  ) => {
    const addon = props.addon
    if (!addon) return undefined
    const target = options.instanceTarget?.(instanceId)
    // A link's file that does not fit this instance installs the instance's
    // own version instead (2.0.40's path), and never replaces an installed copy.
    const fileId = effectiveFileId(
      props.fileId,
      props.linkedFile,
      addon.type,
      target
    )
    const staged = installsAsDatapack(instanceId)
    const owner = Symbol()
    attempts.set(instanceId, owner)
    return {
      owner,
      instanceId,
      fileId,
      addonType: addon.type,
      title: addon.title,
      staged,
      // The core's own pick of a latest version takes a Modrinth project for
      // the mod it is typed as, so its data pack build is picked here, for
      // the instance's game version once that is known.
      datapackBuild:
        !fileId && staged && addon.platform === "modrinth" && target?.mc_version
          ? { projectId: addon.id.toString(), gameVersion: target.mc_version }
          : undefined,
      installLatest: !fileId,
      modSource: fileId ? createModInstallObj(addon, fileId) : undefined,
      latestModSource: !fileId ? createLatestModInstallObj(addon) : undefined,
      replacesMod: fileId ? replacesMod : null
    }
  }
  type InstallAttempt = NonNullable<ReturnType<typeof captureInstallAttempt>>
  const current = (attempt: InstallAttempt) =>
    active && attempts.get(attempt.instanceId) === attempt.owner

  /** `attempt` as it is sent to the core: one for a Modrinth data pack build
   *  names the newest such build for the game version. Undefined, after
   *  saying why, when the project has none or they could not be listed. */
  const resolveInstallAttempt = async (
    attempt: InstallAttempt
  ): Promise<InstallAttempt | undefined> => {
    const wanted = attempt.datapackBuild
    if (!wanted) return attempt
    let build
    try {
      build = newestDatapackBuild(
        await ctx.client.query([
          "modplatforms.modrinth.getProjectVersions",
          {
            project_id: wanted.projectId,
            game_versions: [wanted.gameVersion],
            loaders: ["datapack"]
          }
        ])
      )
    } catch (error) {
      toast.error(
        addonInstallFailure(
          t,
          attempt.title || t("notifications:_trn_addon_fallback_name")
        ),
        { description: extractErrorDisplay(error) }
      )
      return undefined
    }
    if (!build) {
      toast.error(
        t("content:_trn_worlds.no_datapack_for_version", {
          version: wanted.gameVersion,
          interpolation: { escapeValue: false }
        })
      )
      return undefined
    }
    return {
      ...attempt,
      fileId: build.id,
      datapackBuild: undefined,
      installLatest: false,
      modSource: {
        Modrinth: { project_id: wanted.projectId, version_id: build.id }
      },
      latestModSource: undefined
    }
  }

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
                  staged: attempt.staged
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

      const sent = await resolveInstallAttempt(attempt)
      if (!sent) {
        if (current(attempt)) clearInstanceLoadingState(instanceId)
        return
      }

      let taskId: number

      if (sent.installLatest) {
        taskId = await perInstanceInstallLatestMutation.mutateAsync({
          instance_id: instanceId,
          mod_source: sent.latestModSource!
        })
      } else {
        const replacesMod = sent.replacesMod

        taskId = await perInstanceInstallMutation.mutateAsync({
          mod_source: sent.modSource!,
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
          staged: attempt.staged
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

  /** A data pack picked for one world of an instance in the dropdown. */
  const handleWorldSelection = async (
    instanceId: number,
    world: ReadableWorld
  ) => {
    const addon = props.addon
    if (!addon) return
    const fileId = effectiveFileId(
      props.fileId,
      props.linkedFile,
      addon.type,
      options.instanceTarget?.(instanceId)
    )
    const owner = Symbol()
    attempts.set(instanceId, owner)
    const isCurrent = () => active && attempts.get(instanceId) === owner
    setInstanceLoadingStates((prev) => new Map(prev).set(instanceId, true))
    try {
      const taskId = await installWorldDatapackMutation.mutateAsync(
        worldDatapackInstall(addon, fileId, instanceId, world)
      )
      if (!isCurrent()) return
      setInstanceTasks((prev) =>
        new Map(prev).set(instanceId, {
          taskId,
          title: addon.title,
          staged: false,
          world: world.name
        })
      )
    } catch {
      // The core's refusal is shown by the global MutationCache.onError.
      if (isCurrent()) clearInstanceLoadingState(instanceId)
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
    installWorldDatapackMutation,
    handleInstanceSelection,
    handleWorldSelection,
    handleServerInstall,
    clearInstanceLoadingState,
    captureInstallAttempt,
    resolveInstallAttempt,
    installsAsDatapack,
    maybeOpenShaderWizard
  }
}
