import {
  addonDownloadingToServer,
  addonInstallCompletion,
  addonInstallFailure
} from "./addonInstallScope"
import { rspc } from "@/utils/rspcClient"
import {
  Switch,
  Match,
  createSignal,
  createEffect,
  createMemo,
  batch,
  onCleanup
} from "solid-js"
import {
  FEUnifiedSearchResult,
  Mod,
  Progress,
  ServerAddon
} from "@gd/core_module/bindings"
import { useModInstallation } from "./hooks/useModInstallation"
import { useInstanceSearch } from "./hooks/useInstanceSearch"
import { useTaskProgress } from "./hooks/useTaskProgress"
import { resolveTaskPoll } from "./hooks/resolveTaskPoll"
import { InstanceDropdown } from "./components/InstanceDropdown"
import { InstallButton } from "./components/InstallButton"
import { toast } from "@gd/ui"
import { useTransContext } from "@gd/i18n"
import { worldVersionCopy } from "./worldVersionCopy"
import { findInstalledAddon, isInstalledArtifact } from "@/utils/installedAddon"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import {
  effectiveFileId,
  instanceLinkTarget,
  LinkedFile,
  LinkTarget,
  serverLinkTarget
} from "@/utils/linkedFile"

interface ModDownloadButtonProps {
  fileId?: number | string
  /** Metadata of the file `fileId` pins when it came from an install link
   *  rather than the user's own pick (search rows and the addon page header).
   *  An instance or server install keeps the pin only where the file fits
   *  that target, otherwise it installs the version resolved for it.
   *  Versions-tab rows leave it unset: an explicit pick there always installs
   *  as chosen. */
  linkedFile?: LinkedFile
  addon: FEUnifiedSearchResult | undefined
  onDropdownOpenChange?: (isOpen: boolean) => void
  selectedInstanceId?: number
  selectedInstanceMods?: Mod[]
  selectedServerAddons?: ServerAddon[]
  instanceLocked?: boolean
  selectedServerId?: number
  size?: "small" | "medium" | "large"
  iconOnly?: boolean
  /** Opt-in anchor for the rendered install/download button, forwarded to
   *  `InstallButton`. Left unset everywhere this component can render more
   *  than one instance for different addons at once (search result lists,
   *  version rows) — an anchor there would match every one of them. Set
   *  only by the addon page's primary header button; the page's own sticky
   *  icon-only duplicate of the same addon's button deliberately leaves it
   *  unset too, for the same reason. */
  testId?: string
}

const ModDownloadButton = (props: ModDownloadButtonProps) => {
  const [t] = useTransContext()
  const [acceptedTask, setAcceptedTask] = createSignal<{
    taskId: number
    title: string
    type: FEUnifiedSearchResult["type"]
    server: boolean
    copy: ReturnType<typeof worldVersionCopy>
  }>()
  const [pendingRequest, setPendingRequest] = createSignal(false)
  const [progress, setProgress] = createSignal<number | null>(null)
  const loading = () => pendingRequest() || acceptedTask() !== undefined
  let active = true
  let requestOwner: symbol | undefined
  // The server install the newest click asked for. Clicking it again while
  // it is sent or runs would only install the same addon once more after it
  // (the core runs a server's installs one after another), so that click is
  // ignored.
  let serverRequest: string | undefined
  onCleanup(() => {
    active = false
    requestOwner = undefined
  })

  const globalStore = useGlobalStore()
  const instanceTarget = (instanceId: number | undefined) =>
    instanceLinkTarget(globalStore?.instances?.data, instanceId)
  // Only a link row needs the server's release and loader; the search and
  // addon page already hold this query, so it is served from the cache.
  const serverDetails = rspc.createQuery(() => ({
    queryKey: ["server.getServerDetails", props.selectedServerId ?? 0],
    enabled: !!props.selectedServerId && !!props.linkedFile
  }))
  const serverTarget = () =>
    props.selectedServerId ? serverLinkTarget(serverDetails.data) : undefined

  const {
    instanceLoadingStates,
    instanceTasks,
    installLatestModMutation,
    installModMutation,
    handleInstanceSelection,
    handleServerInstall,
    clearInstanceLoadingState,
    captureInstallAttempt,
    maybeOpenShaderWizard
  } = useModInstallation(props, { instanceTarget, serverTarget })

  // The file an install into the selected instance or server uses: the
  // link's file only where it fits that target.
  const instanceFileId = createMemo(() =>
    effectiveFileId(
      props.fileId,
      props.linkedFile,
      props.addon?.type,
      instanceTarget(props.selectedInstanceId)
    )
  )
  const serverFileId = createMemo(() =>
    effectiveFileId(
      props.fileId,
      props.linkedFile,
      props.addon?.type,
      serverTarget()
    )
  )
  const scopedFileId = () =>
    props.selectedServerId ? serverFileId() : instanceFileId()
  // The game versions the link's file lists, for a target that gets its own
  // version instead; undefined where the pin is kept. Same rule as the install.
  const mismatchFor = (target: LinkTarget | undefined) =>
    props.linkedFile &&
    props.fileId !== undefined &&
    effectiveFileId(
      props.fileId,
      props.linkedFile,
      props.addon?.type,
      target
    ) === undefined
      ? props.linkedFile.gameVersions
      : undefined
  const linkedFileMismatch = (instanceId: number | undefined) =>
    mismatchFor(instanceTarget(instanceId))

  const {
    searchQuery,
    setSearchQuery,
    hoveredInstanceId,
    setHoveredInstanceId,
    filteredInstances,
    shouldVirtualize
  } = useInstanceSearch({ addonType: props.addon?.type })

  useTaskProgress(instanceTasks, clearInstanceLoadingState)

  // Reset progress with the accepted record, before its query can observe an
  // immediate terminal outcome. Only explicit Succeeded certifies completion.
  const [lastProgress, setLastProgress] = createSignal<Progress | null>(null)

  // Handle task progress for single instance button
  const task = rspc.createQuery(() => ({
    queryKey: ["vtask.getTaskOutcome", acceptedTask()?.taskId ?? null],
    enabled: acceptedTask() !== undefined
  }))

  createEffect(() => {
    const accepted = acceptedTask()
    if (!accepted) return

    const { action, nextLastProgress } = resolveTaskPoll(
      task?.data,
      lastProgress(),
      true
    )
    setLastProgress(nextLastProgress)

    if (action.kind === "progress") {
      setProgress(action.percent)
    } else if (action.kind === "failed") {
      toast.error(
        addonInstallFailure(
          t,
          accepted.title || t("notifications:_trn_addon_fallback_name")
        ),
        action.message ? { description: action.message } : undefined
      )
      batch(() => {
        setAcceptedTask(undefined)
        setProgress(null)
        setLastProgress(null)
      })
    } else if (action.kind === "completed" || action.kind === "dismissed") {
      if (action.kind === "completed" && action.showSuccessToast) {
        toast.success(
          accepted.copy
            ? t(accepted.copy.completion)
            : addonInstallCompletion(
                t,
                accepted.title || t("notifications:_trn_addon_fallback_name"),
                accepted.type,
                accepted.server
              ),
          { duration: 2000 }
        )
      }
      batch(() => {
        setAcceptedTask(undefined)
        setProgress(null)
        setLastProgress(null)
      })
    }
  })

  const installedMod = createMemo(() =>
    props.addon
      ? findInstalledAddon(
          props.addon,
          props.selectedServerId
            ? { kind: "server", addons: props.selectedServerAddons ?? [] }
            : { kind: "instance", addons: props.selectedInstanceMods ?? [] },
          scopedFileId(),
          props.linkedFile
        )
      : undefined
  )

  const isInstalled = createMemo(() =>
    isInstalledArtifact(installedMod(), scopedFileId())
  )

  const handleDownload = async () => {
    if (!props.addon) return

    // Newer clicks own UI observation. Entered work may still finish after a
    // replacement or disposal; its late response must not publish over them.
    const beginRequest = (server?: string) => {
      const owner = Symbol()
      requestOwner = owner
      serverRequest = server
      setPendingRequest(true)
      return () => active && requestOwner === owner
    }
    const accept = (
      current: () => boolean,
      taskId: number,
      presentation: Omit<NonNullable<ReturnType<typeof acceptedTask>>, "taskId">
    ) => {
      if (!current()) return
      batch(() => {
        setPendingRequest(false)
        setProgress(null)
        setLastProgress(null)
        setAcceptedTask({ taskId, ...presentation })
      })
    }

    // Server installs use the same retained task progression as instance installs.
    const serverId = props.selectedServerId
    if (serverId) {
      if (isInstalled()) return
      const request = [
        serverId,
        props.addon.platform,
        props.addon.id,
        serverFileId() ?? ""
      ].join(":")
      if (loading() && serverRequest === request) return
      const presentation = {
        title: props.addon.title,
        type: props.addon.type,
        server: true,
        copy: undefined
      }
      const current = beginRequest(request)
      try {
        const acceptedTask = await handleServerInstall(serverId)
        if (acceptedTask === undefined)
          throw new Error("Server install returned no task")
        if (!current()) return
        toast.success(
          addonDownloadingToServer(
            t,
            presentation.title || t("notifications:_trn_addon_fallback_name")
          ),
          { duration: 2000 }
        )
        accept(current, acceptedTask, presentation)
      } catch {
        // Error surfaced via global MutationCache.onError
      } finally {
        if (current()) setPendingRequest(false)
      }
      return
    }

    // Instance install path
    const instanceId = props.selectedInstanceId
    if (!instanceId || isInstalled()) return
    const attempt = captureInstallAttempt(
      instanceId,
      installedMod()?.id || null
    )
    if (!attempt) return
    const copy = worldVersionCopy(
      attempt.addonType,
      attempt.fileId,
      attempt.replacesMod
    )
    const current = beginRequest()
    try {
      if ((await maybeOpenShaderWizard(attempt)) !== "direct") return
      let taskId: number
      if (attempt.installLatest) {
        taskId = await installLatestModMutation.mutateAsync({
          instance_id: instanceId,
          mod_source: attempt.latestModSource!
        })
      } else {
        const replacesMod = attempt.replacesMod

        taskId = await installModMutation.mutateAsync({
          mod_source: attempt.modSource!,
          instance_id: instanceId,
          install_deps: true,
          replaces_mod: replacesMod
        })
      }
      accept(current, taskId, {
        title: attempt.title,
        type: attempt.addonType,
        server: false,
        copy
      })
    } catch {
      // Error surfaced via global MutationCache.onError
    } finally {
      if (current()) setPendingRequest(false)
    }
  }

  return (
    <Switch>
      <Match when={props.selectedServerId}>
        <InstallButton
          loading={loading}
          progress={progress}
          isInstalled={isInstalled}
          instanceLocked={() => false}
          fileId={serverFileId()}
          linkedFileMismatch={mismatchFor(serverTarget())}
          installedMod={installedMod}
          onDownload={handleDownload}
          size={props.size}
          iconOnly={props.iconOnly}
          testId={props.testId}
        />
      </Match>
      <Match when={!props.selectedInstanceId}>
        <InstanceDropdown
          addon={props.addon}
          linkedFileMismatch={linkedFileMismatch}
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
          staged={props.addon?.type === "datapack"}
          copyPresentation={worldVersionCopy(
            props.addon?.type,
            instanceFileId(),
            installedMod()?.id
          )}
          fileId={instanceFileId()}
          linkedFileMismatch={linkedFileMismatch(props.selectedInstanceId)}
          installedMod={installedMod}
          onDownload={handleDownload}
          size={props.size}
          iconOnly={props.iconOnly}
          testId={props.testId}
        />
      </Match>
    </Switch>
  )
}

export default ModDownloadButton
