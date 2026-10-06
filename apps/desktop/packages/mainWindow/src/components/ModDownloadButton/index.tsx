import {
  addonDownloadingToServer,
  addonInstallCompletion,
  addonInstallFailure
} from "./addonInstallScope"
import { rspc } from "@/utils/rspcClient"
import { Switch, Match, createSignal, createEffect, createMemo } from "solid-js"
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
  const [taskId, setTaskId] = createSignal<number | null>(null)
  const [pendingInstall, setPendingInstall] = createSignal(false)
  const [copyRequest, setCopyRequest] =
    createSignal<ReturnType<typeof worldVersionCopy>>()

  const {
    instanceLoadingStates,
    instanceTaskIds,
    installLatestModMutation,
    installModMutation,
    handleInstanceSelection,
    handleServerInstall,
    clearInstanceLoadingState,
    latestModInstallObj,
    modInstallObj,
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

  const { loading, setLoading, progress, setProgress } = useTaskProgress(
    instanceTaskIds,
    clearInstanceLoadingState,
    props.addon
  )

  // Last non-`Indeterminate`/`null` progress seen for the tracked task. Used
  // to tell a genuine completion (progress goes `Known` then the task is
  // forgotten, `data` becomes `null`) apart from a task that failed and was
  // then dismissed (`data` also becomes `null` once forgotten) — only the
  // former should trigger the success toast below. Reset alongside `taskId`
  // whenever a new task starts being tracked (the two effects right below)
  // and when a tracked task is dismissed after failing (the poll effect's
  // `"failed"` branch further down) — otherwise a `Failed` observed for one
  // task would keep suppressing the success toast for whatever task gets
  // tracked next. Mirrors `useTaskProgress.ts`'s per-instance map, which
  // retires its own entry the same way on both failure and completion.
  const [lastProgress, setLastProgress] = createSignal<Progress | null>(null)

  createEffect(() => {
    if (installLatestModMutation.isPending) {
      setLoading(true)
    }

    if (installLatestModMutation.isSuccess) {
      setTaskId(installLatestModMutation.data)
      setLastProgress(null)
    }
  })

  createEffect(() => {
    if (installModMutation.isPending) {
      setLoading(true)
    }

    if (installModMutation.isSuccess) {
      setTaskId(installModMutation.data)
      setLastProgress(null)
    }
  })

  // Handle task progress for single instance button
  const task = rspc.createQuery(() => ({
    queryKey: ["vtask.getTaskOutcome", taskId()],
    enabled: taskId() !== null
  }))

  createEffect(() => {
    if (taskId() === null) return

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
        t("notifications:_trn_addon_install_failed", {
          title:
            props.addon?.title || t("notifications:_trn_addon_fallback_name")
        }),
        action.message ? { description: action.message } : undefined
      )
      setLoading(false)
      setPendingInstall(false)
      setTaskId(null)
      setProgress(null)
      // Retire the failure from `lastProgress` too — otherwise it would
      // outlive this task and suppress the success toast of whatever gets
      // tracked next (see this signal's own doc comment above).
      setLastProgress(null)
    } else if (action.kind === "completed" || action.kind === "dismissed") {
      if (action.kind === "completed" && action.showSuccessToast) {
        toast.success(
          copyRequest()
            ? t(copyRequest()!.completion)
            : addonInstallCompletion(
                props.addon?.title || "Addon",
                props.addon?.type,
                !!props.selectedServerId
              ),
          { duration: 2000 }
        )
      }
      setLoading(false)
      setTaskId(null)
      setProgress(null)
      setPendingInstall(false)
    }
  })

  const installedMod = createMemo(() =>
    props.addon
      ? findInstalledAddon(
          props.addon,
          props.selectedServerId
            ? { kind: "server", addons: props.selectedServerAddons ?? [] }
            : { kind: "instance", addons: props.selectedInstanceMods ?? [] },
          props.fileId
        )
      : undefined
  )

  const isInstalled = createMemo(() =>
    isInstalledArtifact(installedMod(), scopedFileId())
  )

  const handleDownload = async () => {
    if (!props.addon) return

    // Server installs use the same retained task progression as instance installs.
    const serverId = props.selectedServerId
    if (serverId) {
      setLoading(true)
      try {
        const acceptedTask = await handleServerInstall(serverId)
        if (acceptedTask === undefined)
          throw new Error("Server install returned no task")
        setPendingInstall(true)
        setLastProgress(null)
        setTaskId(acceptedTask)
        toast.success(
          t("notifications:_trn_addon_downloading_to_server", {
            title:
              props.addon?.title || t("notifications:_trn_addon_fallback_name")
          }),
          { duration: 2000 }
        )
      } catch {
        // Error surfaced via global MutationCache.onError
        setLoading(false)
        setProgress(null)
      }
      return
    }

    // Instance install path
    const instanceId = props.selectedInstanceId
    if (!instanceId || isInstalled()) return

    if (await maybeOpenShaderWizard(instanceId)) {
      return
    }

    setCopyRequest(
      worldVersionCopy(props.addon.type, props.fileId, installedMod()?.id)
    )
    setPendingInstall(true)
    try {
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
          install_deps: true,
          replaces_mod: replacesMod
        })
      }
    } catch {
      // Error surfaced via global MutationCache.onError
      setPendingInstall(false)
      setLoading(false)
      setProgress(null)
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
