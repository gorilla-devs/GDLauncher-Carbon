import { rspc } from "@/utils/rspcClient"
import { Switch, Match, createSignal, createEffect, createMemo } from "solid-js"
import {
  FEUnifiedSearchResult,
  Mod,
  ServerAddon
} from "@gd/core_module/bindings"
import { useModInstallation } from "./hooks/useModInstallation"
import { useInstanceSearch } from "./hooks/useInstanceSearch"
import { useTaskProgress } from "./hooks/useTaskProgress"
import { InstanceDropdown } from "./components/InstanceDropdown"
import { InstallButton } from "./components/InstallButton"
import { toast } from "@gd/ui"
import { useTransContext } from "@gd/i18n"

interface ModDownloadButtonProps {
  fileId?: number | string
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
    if (!props.addon) return undefined

    // Check server addons first (when browsing for a server)
    const serverAddons = props.selectedServerAddons
    if (serverAddons && serverAddons.length > 0) {
      // Match by platform project ID (reliable, from metadata)
      const byId = serverAddons.find((addon) => {
        if (props.addon!.platform === "curseforge") {
          return (
            addon.curseforgeProjectId !== null &&
            addon.curseforgeProjectId.toString() === props.addon!.id.toString()
          )
        } else if (props.addon!.platform === "modrinth") {
          return addon.modrinthProjectId === props.addon!.id.toString()
        }
        return false
      })
      if (byId) return { id: byId.id } as any

      // Fallback: match by slug against display name
      const slug = props.addon.slug?.toLowerCase()
      if (slug) {
        const bySlug = serverAddons.find((addon) => {
          const name = addon.displayName.toLowerCase()
          return name === slug || name.startsWith(slug + "-")
        })
        if (bySlug) return { id: bySlug.id } as any
      }

      return undefined
    }

    // Check instance mods
    const mods = props.selectedInstanceMods || []
    const found = mods.find((mod) => {
      if (props.addon!.platform === "curseforge") {
        return (
          mod.curseforge?.project_id ===
          parseInt(props.addon!.id.toString(), 10)
        )
      } else if (props.addon!.platform === "modrinth") {
        return mod.modrinth?.project_id === props.addon!.id.toString()
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

    // Server install path — fire-and-forget, no task tracking
    const serverId = props.selectedServerId
    if (serverId) {
      setLoading(true)
      try {
        await handleServerInstall(serverId)
        toast.success(
          t("notifications:_trn_addon_downloading_to_server", {
            title:
              props.addon?.title || t("notifications:_trn_addon_fallback_name")
          }),
          { duration: 2000 }
        )
      } catch {
        // Error surfaced via global MutationCache.onError
      } finally {
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

    // KNOWN BUG (worlds only): `isWorld && taskId() === null` conflates "the
    // install has not started yet" with "the install finished". `taskId()`
    // reads `null` both before `installLatestModMutation` resolves with a real
    // one and after the vtask poll clears it on completion, and this effect
    // cannot tell those apart. `handleDownload` flips `loading` true
    // synchronously, which re-runs this effect while `taskId()` is still null
    // from before the mutation resolved, so the branch below immediately flips
    // it back to false — a world install shows no in-progress feedback at all.
    // Confirmed live: the spinner never becomes visible even briefly while a
    // world downloads and extracts correctly (`installModIntoInstance`'s doc
    // comment, `apps/desktop/e2e-tests/helpers/mods.ts`, records the
    // measurement and the completion signal the e2e suite uses instead).
    // Fixing it needs a real "started" signal distinct from `taskId`, not a
    // tweak to this condition.
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
      <Match when={props.selectedServerId}>
        <InstallButton
          loading={loading}
          progress={progress}
          isInstalled={isInstalled}
          instanceLocked={() => false}
          fileId={props.fileId}
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
          testId={props.testId}
        />
      </Match>
    </Switch>
  )
}

export default ModDownloadButton
