import {
  addonDownloadingToServer,
  addonInstallCompletion,
  addonInstallFailure
} from "./addonInstallScope"
import { rspc } from "@/utils/rspcClient"
import {
  Switch,
  Match,
  Show,
  createSignal,
  createEffect,
  createMemo,
  batch,
  on,
  onCleanup,
  untrack
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
import { InstallPlacesStep } from "./components/InstallPlacesStep"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuTrigger,
  toast
} from "@gd/ui"
import { useTransContext } from "@gd/i18n"
import { worldVersionCopy } from "./worldVersionCopy"
import { findInstalledAddon, isInstalledArtifact } from "@/utils/installedAddon"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import useSearchContext from "@/components/SearchInputContext"
import { LOCAL_ERROR_TOAST } from "@/utils/mutationErrorToast"
import { refreshWorldsAfter } from "@/utils/refusalText"
import { worldLabel } from "@/utils/worldLabel"
import {
  asksWhichWorld,
  canBeDatapack,
  installedInInstance,
  isDatapackBuild,
  readInstallPlaces,
  ReadableWorld,
  readableWorld,
  worldDatapackInstall,
  worldTargetName,
  WORLDS_STALE_TIME
} from "@/utils/worldTarget"
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
  /** A world of the selected instance (its folder name) that a project with
   *  a data pack build installs into instead of the instance. Ignored for any
   *  other addon. */
  selectedWorld?: string
  /** `fileId` is a version picked from the Versions tab's list: it installs
   *  into the selected instance at once, never asking which world. */
  pickedVersion?: boolean
  /** Whether the Modrinth version `fileId` names is a data pack build (a zip
   *  tagged "datapack") rather than a mod jar, when the caller has that
   *  version at hand. Left unset, the button reads the version where the
   *  project ships both. */
  fileIsDatapackBuild?: boolean
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
    /** A data pack that lands in the instance, to be added to a world there. */
    staged: boolean
    server: boolean
    copy: ReturnType<typeof worldVersionCopy>
    world?: string
    instanceId?: number
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
  const ctx = rspc.useContext()
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

  const worldName = () =>
    props.selectedServerId
      ? undefined
      : worldTargetName(
          props.selectedInstanceId,
          props.addon,
          props.selectedWorld
        )
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
  // A Modrinth project that ships a data pack lists its mod jars beside its
  // data pack builds: a version of it picked by hand is one or the other by
  // itself. A link only names the version, so it is read here.
  const readsVersion = () =>
    props.fileIsDatapackBuild === undefined &&
    props.fileId !== undefined &&
    props.fileId !== "" &&
    props.addon?.platform === "modrinth" &&
    props.addon.type !== "datapack" &&
    canBeDatapack(props.addon)
  const linkedVersion = rspc.createQuery(() => ({
    queryKey: ["modplatforms.modrinth.getVersion", String(props.fileId ?? "")],
    enabled: readsVersion(),
    // Unread, the version replaces no installed copy; nothing to tell.
    meta: LOCAL_ERROR_TOAST
  }))
  // Until the link's version is read nobody knows which copy in the
  // instance it replaces, so a click then would put a second copy beside it.
  // A world or a server is not compared by kind and does not wait.
  const versionUnread = () =>
    readsVersion() &&
    !props.selectedServerId &&
    worldName() === undefined &&
    linkedVersion.data === undefined &&
    !linkedVersion.isError
  const fileIsDatapackBuild = () =>
    props.fileIsDatapackBuild ??
    (readsVersion() && linkedVersion.data
      ? isDatapackBuild(linkedVersion.data)
      : undefined)

  const searchContext = useSearchContext()
  // A data pack for an instance with no world chosen asks where in it. A
  // picked mod jar does not: no world can take it.
  const asksWhere = () =>
    !props.selectedServerId &&
    !!props.selectedInstanceId &&
    !props.pickedVersion &&
    worldName() === undefined &&
    !(instanceFileId() !== undefined && fileIsDatapackBuild() === false) &&
    asksWhichWorld(props.addon, searchContext?.searchQuery().projectType)
  // Search rows share this query, and the identity it carries lets the core
  // refuse an install into a world that changed since it was read. Read only
  // to offer the worlds, a failure leaves the instance to install into, so
  // it needs no toast.
  const worlds = rspc.createQuery(() => ({
    queryKey: ["instance.getWorlds", props.selectedInstanceId ?? 0],
    enabled: worldName() !== undefined || asksWhere(),
    ...(worldName() === undefined && {
      staleTime: WORLDS_STALE_TIME,
      meta: LOCAL_ERROR_TOAST
    })
  }))
  const targetWorld = createMemo(() => {
    const name = worldName()
    return name === undefined
      ? undefined
      : (worlds.data ?? []).find((world) => world.name === name)
  })
  const worldUnavailable = () =>
    worldName() !== undefined && !readableWorld(targetWorld())

  const {
    instanceLoadingStates,
    instanceTasks,
    installLatestModMutation,
    installModMutation,
    installWorldDatapackMutation,
    handleInstanceSelection,
    handleWorldSelection,
    handleServerInstall,
    clearInstanceLoadingState,
    captureInstallAttempt,
    resolveInstallAttempt,
    instanceInstallFor,
    maybeOpenShaderWizard
  } = useModInstallation(props, {
    instanceTarget,
    serverTarget,
    fileIsDatapackBuild
  })

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
  } = useInstanceSearch({ addon: () => props.addon })

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
      refreshWorldsAfter(action.code, ctx.queryClient, accepted.instanceId)
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
                accepted.staged,
                accepted.server,
                accepted.world
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
            : worldName() !== undefined
              ? { kind: "world", datapacks: targetWorld()?.datapacks ?? [] }
              : {
                  kind: "instance",
                  addons: props.selectedInstanceMods ?? [],
                  install: instanceInstallFor(props.selectedInstanceId)
                },
          scopedFileId(),
          props.linkedFile
        )
      : undefined
  )

  const holdsRequested = () =>
    isInstalledArtifact(installedMod(), scopedFileId())
  // What the "where" step offers; undefined while the worlds are read.
  const places = createMemo(() =>
    asksWhere()
      ? readInstallPlaces(props.addon, worlds, holdsRequested())
      : undefined
  )
  const isInstalled = createMemo(() =>
    installedInInstance(holdsRequested(), asksWhere(), places())
  )

  /** Installs into the selected target: the server, the world of the search,
   *  `into` when the "where" step picked a world, or else the instance. */
  const handleDownload = async (into?: ReadableWorld) => {
    if (!props.addon || versionUnread()) return

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
        staged: false,
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

    // World install path: never falls back to the instance.
    if (into || worldName() !== undefined) {
      const world = into ?? readableWorld(targetWorld())
      if (!world || loading()) return
      const presentation = {
        title: props.addon.title,
        staged: false,
        server: false,
        copy: undefined,
        world: worldLabel(world),
        instanceId
      }
      const request = worldDatapackInstall(
        props.addon,
        instanceFileId(),
        instanceId,
        world
      )
      const current = beginRequest()
      try {
        const taskId = await installWorldDatapackMutation.mutateAsync(request)
        accept(current, taskId, presentation)
      } catch {
        // Error surfaced via global MutationCache.onError
      } finally {
        if (current()) setPendingRequest(false)
      }
      return
    }
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
      const sent = await resolveInstallAttempt(attempt)
      if (!sent) return
      let taskId: number
      if (sent.installLatest) {
        taskId = await installLatestModMutation.mutateAsync({
          instance_id: instanceId,
          mod_source: sent.latestModSource!
        })
      } else {
        const replacesMod = sent.replacesMod

        taskId = await installModMutation.mutateAsync({
          mod_source: sent.modSource!,
          instance_id: instanceId,
          install_deps: true,
          replaces_mod: replacesMod
        })
      }
      accept(current, taskId, {
        title: attempt.title,
        staged: attempt.staged,
        server: false,
        copy
      })
    } catch {
      // Error surfaced via global MutationCache.onError
    } finally {
      if (current()) setPendingRequest(false)
    }
  }

  // The "where" step under Download: "open" while its menu shows, "waiting"
  // after a click made before the worlds were read.
  const [where, setWhere] = createSignal<"closed" | "waiting" | "open">(
    "closed"
  )
  const showWhere = (next: ReturnType<typeof where>) => {
    const inUse = where() !== "closed"
    setWhere(next)
    if (inUse !== (next !== "closed"))
      props.onDropdownOpenChange?.(next !== "closed")
  }
  // Only with something to choose is Download the trigger of a menu: a
  // trigger acts on pointer down, where a plain button waits for the click.
  const offersChoice = () => !versionUnread() && places()?.offers === "choice"
  /** A click on the plain button. Before the worlds are read nobody knows
   *  whether there is a choice, so the click waits for them. */
  const handlePlainClick = () => {
    if (versionUnread()) return
    if (!asksWhere() || places()) return handleDownload()
    if (!props.instanceLocked && !loading() && !isInstalled())
      showWhere("waiting")
  }
  // A waiting click or an open step belongs to the target it was made for.
  createEffect(
    on(
      () => [props.selectedInstanceId, props.selectedServerId, props.addon?.id],
      () => showWhere("closed"),
      { defer: true }
    )
  )
  onCleanup(() => {
    if (where() !== "closed") props.onDropdownOpenChange?.(false)
  })
  // A click that waited gets its answer once the worlds are read: the step,
  // or with no readable world the install into the instance. An open step
  // only stays while it has something to choose.
  createEffect(() => {
    const now = where()
    if (now === "closed") return
    const offered = places()
    if (now === "waiting" && asksWhere() && !offered) return
    if (offered?.offers === "choice") return showWhere("open")
    untrack(() => {
      showWhere("closed")
      if (now === "waiting" && offered?.offers === "instance")
        void handleDownload()
    })
  })

  const instanceButton = (onDownload: () => void) => (
    <InstallButton
      loading={() => loading() || where() === "waiting" || versionUnread()}
      progress={progress}
      isInstalled={isInstalled}
      instanceLocked={() => props.instanceLocked ?? false}
      unavailable={worldUnavailable()}
      // Installed once its worlds are known means no world is left to take
      // the pack, so "add it to a world" would point nowhere.
      staged={
        instanceInstallFor(props.selectedInstanceId) === "datapack" &&
        worldName() === undefined &&
        !(places() && isInstalled())
      }
      copyPresentation={worldVersionCopy(
        props.addon?.type,
        instanceFileId(),
        installedMod()?.id
      )}
      fileId={instanceFileId()}
      linkedFileMismatch={linkedFileMismatch(props.selectedInstanceId)}
      installedMod={installedMod}
      onDownload={onDownload}
      size={props.size}
      iconOnly={props.iconOnly}
      testId={props.testId}
    />
  )

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
          onDownload={() => handleDownload()}
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
          handleWorldSelection={handleWorldSelection}
          picked={
            props.fileId !== undefined && props.fileId !== ""
              ? { datapackBuild: fileIsDatapackBuild() }
              : undefined
          }
          onDropdownOpenChange={props.onDropdownOpenChange}
          size={props.size}
          iconOnly={props.iconOnly}
        />
      </Match>
      <Match when={props.selectedInstanceId}>
        <Show when={offersChoice()} fallback={instanceButton(handlePlainClick)}>
          {/* The menu's trigger takes the click, so the button itself does
              nothing. */}
          <DropdownMenu
            open={where() === "open"}
            onOpenChange={(open) =>
              showWhere(
                open && !props.instanceLocked && !loading() ? "open" : "closed"
              )
            }
          >
            <DropdownMenuTrigger class="b-0 bg-transparent p-0">
              {instanceButton(() => {})}
            </DropdownMenuTrigger>
            <DropdownMenuContent class="w-80 p-0">
              <InstallPlacesStep
                places={places()}
                onWorld={(world) => {
                  showWhere("closed")
                  void handleDownload(world)
                }}
                onInstance={() => {
                  showWhere("closed")
                  void handleDownload()
                }}
              />
            </DropdownMenuContent>
          </DropdownMenu>
        </Show>
      </Match>
    </Switch>
  )
}

export default ModDownloadButton
