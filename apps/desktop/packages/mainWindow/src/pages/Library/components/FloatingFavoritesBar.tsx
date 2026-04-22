import {
  createMemo,
  createEffect,
  createSignal,
  onCleanup,
  onMount,
  For,
  Show
} from "solid-js"
import { Portal } from "solid-js/web"
import { Trans, useTransContext, type TypedTFunction } from "@gd/i18n"
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuGroup,
  ContextMenuGroupLabel,
  ContextMenuItem,
  ContextMenuPortal,
  ContextMenuSeparator,
  ContextMenuSub,
  ContextMenuSubContent,
  ContextMenuSubTrigger,
  ContextMenuTrigger,
  Spinner,
  Tooltip,
  TooltipContent,
  TooltipTrigger
} from "@gd/ui"
import { rspc } from "@/utils/rspcClient"
import { useGDNavigate } from "@/managers/NavigationManager"
import { useModal } from "@/managers/ModalsManager"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { getInstanceImageUrl } from "@/utils/instances"
import { getModloaderIcon } from "@/utils/sidebar"
import { setClickedInstanceId } from "@/components/InstanceTile"
import {
  setExportStep,
  setPayload
} from "@/managers/ModalsManager/modals/InstanceExport"
import { setCheckedFiles } from "@/managers/ModalsManager/modals/InstanceExport/atoms/ExportCheckboxParent"
import useSearchContext from "@/components/SearchInputContext"
import GdlFeatureContextMenuItem from "@/components/GdlFeatureContextMenuItem"
import { useDragContext } from "../DragContext"
import { LibraryMode } from "../types"
import DefaultImg from "/assets/images/default-instance-img.png"
import type { ValidListInstance, ListServer } from "@gd/core_module/bindings"

/** Icon + text hint centered over the library grid area */
function UnfavoriteHint() {
  const [pos, setPos] = createSignal<{ left: string; top: string } | null>(null)

  onMount(() => {
    const el = document.querySelector<HTMLElement>(
      "[style*='view-transition-name: library-content']"
    )
    if (el) {
      const rect = el.getBoundingClientRect()
      setPos({
        left: `${rect.left + rect.width / 2}px`,
        top: `${rect.top + rect.height / 2}px`
      })
    }
  })

  return (
    <Show when={pos()}>
      {(p) => (
        <div
          class="fixed flex flex-col items-center gap-3 -translate-x-1/2 -translate-y-1/2"
          style={{ left: p().left, top: p().top }}
        >
          <div class="i-ri:star-off-line text-white/90 h-12 w-12" />
          <span class="text-white/90 text-sm font-medium">
            <Trans key="instances:_trn_drop_to_unfavorite" />
          </span>
        </div>
      )}
    </Show>
  )
}

function formatRelativeTime(
  t: TypedTFunction,
  dateString: string | null | undefined
): string {
  if (!dateString) return t("instances:_trn_never_played")
  const date = new Date(dateString)
  const now = new Date()
  const diffMs = now.getTime() - date.getTime()
  const diffMins = Math.floor(diffMs / 60000)
  const diffHours = Math.floor(diffMs / 3600000)
  const diffDays = Math.floor(diffMs / 86400000)

  if (diffMins < 1) return t("instances:_trn_just_now")
  if (diffMins < 60) return t("instances:_trn_minutes_ago", { count: diffMins })
  if (diffHours < 24) return t("instances:_trn_hours_ago", { count: diffHours })
  if (diffDays < 30) return t("instances:_trn_days_ago", { count: diffDays })
  return date.toLocaleDateString()
}

function formatPlaytime(
  t: TypedTFunction,
  seconds: number | undefined
): string {
  if (!seconds || seconds === 0) return t("instances:_trn_no_playtime")
  const hours = seconds / 3600
  if (hours < 0.1) return t("instances:_trn_less_than_hour")
  return t("instances:_trn_hours_played", {
    count: parseFloat(hours.toFixed(1))
  })
}

interface FloatingFavoritesBarProps {
  favoriteIds: number[]
  isSelectionActive: boolean
  libraryMode: LibraryMode
}

// Module-level set that accumulates all favorite IDs we've ever seen,
// so the expand animation only fires on first app load and for genuinely
// new favorites — not on page switches or instance/server mode toggles.
let knownFavoriteIds: Set<number> | null = null

export function FloatingFavoritesBar(props: FloatingFavoritesBarProps) {
  const dragContext = useDragContext()
  const globalStore = useGlobalStore()
  let containerRef: HTMLDivElement | undefined

  // Track newly added favorites for entrance animation
  const [newlyAddedIds, setNewlyAddedIds] = createSignal<Set<number>>(new Set())
  const [recentlyAdded, setRecentlyAdded] = createSignal(false)

  createEffect(() => {
    const current = props.favoriteIds
    if (knownFavoriteIds === null) {
      // First time ever — treat all as new (triggers expand on app load)
      knownFavoriteIds = new Set(current)
      if (current.length > 0) {
        setNewlyAddedIds(new Set(current))
        setRecentlyAdded(true)
        setTimeout(() => {
          setNewlyAddedIds(new Set<number>())
          setRecentlyAdded(false)
        }, 600)
      }
      return
    }
    const added = current.filter((id) => !knownFavoriteIds!.has(id))
    // Accumulate — never forget IDs we've seen
    for (const id of current) knownFavoriteIds.add(id)
    if (added.length > 0) {
      setNewlyAddedIds(new Set(added))
      setRecentlyAdded(true)
      setTimeout(() => {
        setNewlyAddedIds(new Set<number>())
        setRecentlyAdded(false)
      }, 600)
    }
  })

  const isOver = createMemo(() => {
    const target = dragContext.dropTarget()
    return target?.type === "favorites"
  })

  const allDraggedAreFavorite = createMemo(() => {
    const draggedIds = dragContext.draggedIds()
    if (draggedIds.length === 0) return false
    if (props.libraryMode === "servers") {
      const servers = globalStore.servers.data || []
      const draggedServers = servers.filter((s) => draggedIds.includes(s.id))
      return draggedServers.every((s) => s.favorite)
    }
    const instances = globalStore.instances.data || []
    const draggedInstances = instances.filter((i) => draggedIds.includes(i.id))
    return draggedInstances.every((i) => i.favorite)
  })

  const isDragActiveForItems = createMemo(
    () =>
      dragContext.isDragging() &&
      (dragContext.dragType() === "instance" ||
        (props.libraryMode === "servers" &&
          dragContext.dragType() === "server"))
  )

  // Register drop zone when dragging instances
  // When dragging FROM favorites, scope all zones to "favorites-drag" so grid zones are ignored
  createEffect(() => {
    if (!isDragActiveForItems()) {
      dragContext.unregisterDropZone("floating-favorites-bar")
      return
    }

    const el = containerRef
    if (!el) return

    const fromFavorites = dragContext.getDragOrigin() === "favorites"

    if (fromFavorites) {
      dragContext.setActiveScope("favorites-drag")
    }

    const rect = el.getBoundingClientRect()
    dragContext.registerDropZone({
      id: "floating-favorites-bar",
      rect,
      element: el,
      target: { type: "favorites" },
      scope: fromFavorites ? "favorites-drag" : undefined
    })

    onCleanup(() => {
      if (fromFavorites) {
        dragContext.setActiveScope(null)
      }
    })
  })

  onCleanup(() => {
    dragContext.unregisterDropZone("floating-favorites-bar")
  })

  const showBar = createMemo(
    () =>
      (props.favoriteIds.length > 0 || isDragActiveForItems()) &&
      !props.isSelectionActive
  )

  // Signal only drives child stagger — container sizing is pure CSS group-hover
  const [isExpanded, setIsExpanded] = createSignal(false)
  const [openMenuCount, setOpenMenuCount] = createSignal(0)

  const expanded = () =>
    isExpanded() ||
    isDragActiveForItems() ||
    openMenuCount() > 0 ||
    recentlyAdded()

  const isDraggingFromFavorites = createMemo(
    () =>
      dragContext.isDragging() && dragContext.getDragOrigin() === "favorites"
  )

  return (
    <Show when={showBar()}>
      {/* Backdrop overlay when dragging out of favorites — like BackdropDropZone in folders */}
      <Show when={isDraggingFromFavorites()}>
        <Portal>
          <div class="fixed inset-0 z-30 pointer-events-auto">
            <div class="absolute inset-0 bg-black/50" />
            <div class="absolute inset-0 bg-primary-500/20 border-2 border-dashed border-primary-500" />
            <Show when={!isOver()}>
              <UnfavoriteHint />
            </Show>
          </div>
        </Portal>
      </Show>
      <Portal>
        <div
          class="group/dock fixed bottom-6 left-6 flex animate-popoverEnter"
          classList={{
            "z-[10001]": isDragActiveForItems(),
            "z-40": !isDragActiveForItems()
          }}
          onMouseEnter={() => setIsExpanded(true)}
          onMouseLeave={() => setIsExpanded(false)}
        >
          <div
            ref={containerRef}
            class="relative flex items-center min-h-13 min-w-13 rounded-full shadow-lg transition-all duration-250 ease-spring cursor-pointer overflow-visible group-hover/dock:py-3 group-hover/dock:pr-5"
            classList={{
              "bg-darkSlate-800 border border-white/10 shadow-darkSlate-900/50": true,
              "py-3 pr-5":
                isDragActiveForItems() || openMenuCount() > 0 || recentlyAdded()
            }}
          >
            {/* Overlay on bar — always rendered, animated via opacity */}
            <div
              class="absolute inset-0 rounded-full z-10 border-2 border-solid transition-all duration-250 ease-spring"
              classList={{
                "opacity-100 pointer-events-auto": isOver(),
                "opacity-0 pointer-events-none": !isOver(),
                "border-red-500 bg-red-500/60": allDraggedAreFavorite(),
                "border-primary-500 bg-primary-500/60": !allDraggedAreFavorite()
              }}
            />

            {/* Star / state icon - fixed w-13 container so position doesn't shift */}
            <div class="flex items-center justify-center w-13 h-13 shrink-0">
              <div
                class="h-6 w-6 transition-all duration-250 ease-spring"
                classList={{
                  "i-ri:add-line text-primary-400 scale-125":
                    isOver() && !allDraggedAreFavorite(),
                  "i-ri:forbid-line text-red-400 scale-125":
                    isOver() && allDraggedAreFavorite(),
                  "i-ri:star-fill text-yellow-500": !isOver()
                }}
              />
            </div>

            {/* Drop feedback during drag (no favorites yet) */}
            <Show
              when={isDragActiveForItems() && props.favoriteIds.length === 0}
            >
              <span class="text-sm text-lightSlate-300 whitespace-nowrap px-2">
                <Trans key="instances:_trn_drop_to_favorite" />
              </span>
            </Show>

            {/* Add to favorites text — always rendered, animated */}
            <span
              class="absolute inset-0 flex items-center justify-center text-sm text-white font-medium whitespace-nowrap z-20 pointer-events-none transition-all duration-250 ease-spring"
              classList={{
                "opacity-100 scale-100": isOver() && !allDraggedAreFavorite(),
                "opacity-0 scale-75": !isOver() || allDraggedAreFavorite()
              }}
            >
              <Trans key="instances:_trn_drop_to_favorite" />
            </span>

            {/* Already favorite text — always rendered, animated */}
            <span
              class="absolute inset-0 flex items-center justify-center text-sm text-white font-medium whitespace-nowrap z-20 pointer-events-none transition-all duration-250 ease-spring"
              classList={{
                "opacity-100 scale-100": isOver() && allDraggedAreFavorite(),
                "opacity-0 scale-75": !isOver() || !allDraggedAreFavorite()
              }}
            >
              <Trans key="instances:_trn_already_favorite" />
            </span>

            {/* Avatar row - CSS group-hover drives sizing, JS signal drives child stagger */}
            <Show when={props.favoriteIds.length > 0}>
              <div
                class="flex items-center gap-3 max-w-0 opacity-0 pointer-events-none group-hover/dock:max-w-[70vw] group-hover/dock:opacity-100 group-hover/dock:pointer-events-auto transition-all duration-250 ease-spring"
                classList={{
                  "!max-w-[70vw] !opacity-100 !pointer-events-auto":
                    isDragActiveForItems() ||
                    openMenuCount() > 0 ||
                    recentlyAdded()
                }}
                style={{
                  "scrollbar-width": "none",
                  "clip-path": "inset(-8px -8px -8px -8px)",
                  overflow: "visible"
                }}
              >
                <For each={props.favoriteIds}>
                  {(instanceId, index) => (
                    <DockAvatar
                      instanceId={instanceId}
                      isDragActive={isDragActiveForItems()}
                      expanded={expanded()}
                      index={index()}
                      isNewlyAdded={newlyAddedIds().has(instanceId)}
                      onMenuOpenChange={(open) =>
                        setOpenMenuCount((c) => c + (open ? 1 : -1))
                      }
                      libraryMode={props.libraryMode}
                    />
                  )}
                </For>
              </div>
            </Show>
          </div>
        </div>
      </Portal>
    </Show>
  )
}

interface DockAvatarProps {
  instanceId: number
  isDragActive: boolean
  expanded: boolean
  index: number
  isNewlyAdded: boolean
  onMenuOpenChange: (open: boolean) => void
  libraryMode: LibraryMode
}

function DockAvatar(props: DockAvatarProps) {
  const [t] = useTransContext()
  const navigate = useGDNavigate()
  const modalsContext = useModal()
  const globalStore = useGlobalStore()
  const searchContext = useSearchContext()
  const dragContext = useDragContext()
  const [isMenuOpen, setIsMenuOpen] = createSignal(false)

  const isServerMode = () => props.libraryMode === "servers"

  const isBeingDragged = createMemo(
    () =>
      dragContext.isDragging() &&
      dragContext.getDragOrigin() === "favorites" &&
      dragContext.draggedIds().includes(props.instanceId)
  )

  const handlePointerDown = (e: PointerEvent) => {
    // Only left button, skip if context menu
    if (e.button !== 0) return
    const type = isServerMode() ? "server" : "instance"
    dragContext.startDrag(type, [props.instanceId], e, "favorites")
  }

  // --- Instance mode data ---
  const instance = createMemo(() =>
    isServerMode()
      ? undefined
      : globalStore.instances.data?.find(
          (i) => (i.id as unknown as number) === props.instanceId
        )
  )

  const validInstance = (): ValidListInstance | undefined =>
    instance()?.status.status === "valid"
      ? (instance()?.status.value as ValidListInstance)
      : undefined

  const instanceImageUrl = createMemo(() => {
    const inst = instance()
    return inst?.icon_revision
      ? getInstanceImageUrl(props.instanceId, inst.icon_revision)
      : undefined
  })

  const modloader = () => validInstance()?.modloader ?? null
  const modloaderVersion = () => validInstance()?.modloader_version ?? null
  const mcVersion = () => validInstance()?.mc_version ?? null
  const lastPlayed = createMemo(() =>
    formatRelativeTime(t, instance()?.last_played)
  )
  const playtime = createMemo(() =>
    formatPlaytime(t, instance()?.seconds_played)
  )

  // --- Server mode data ---
  const server = createMemo((): ListServer | undefined =>
    isServerMode()
      ? globalStore.servers.data?.find((s) => s.id === props.instanceId)
      : undefined
  )

  const serverStatus = () => server()?.state.status
  const isServerRunning = () => serverStatus() === "running"

  // --- Unified accessors ---
  const itemName = () => (isServerMode() ? server()?.name : instance()?.name)
  const itemExists = () => (isServerMode() ? !!server() : !!instance())
  const isFavorite = () =>
    isServerMode() ? server()?.favorite : instance()?.favorite

  // Instance mutations
  const launchInstanceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.launchInstance"]
  }))

  const killInstanceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.killInstance"]
  }))

  const openFolderMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.openInstanceFolder"]
  }))

  const duplicateInstanceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.duplicateInstance"]
  }))

  const instanceSetFavoriteMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.setFavorite"]
  }))

  // Server mutations
  const startServerMutation = rspc.createMutation(() => ({
    mutationKey: ["server.startServer"]
  }))

  const stopServerMutation = rspc.createMutation(() => ({
    mutationKey: ["server.stopServer"]
  }))

  const serverSetFavoriteMutation = rspc.createMutation(() => ({
    mutationKey: ["server.setFavorite"]
  }))

  // State checks
  const instanceState = createMemo(() => validInstance()?.state?.state)
  const isRunning = () =>
    isServerMode() ? isServerRunning() : instanceState() === "running"
  const isQueued = () => !isServerMode() && instanceState() === "queued"
  const isPreparing = () => !isServerMode() && instanceState() === "preparing"
  const isDeleting = () =>
    isServerMode()
      ? serverStatus() === "deleting"
      : instanceState() === "deleting"
  const isLoading = () => isQueued() || isPreparing()
  const isBusy = () =>
    isServerMode()
      ? serverStatus() === "starting" || serverStatus() === "stopping"
      : isLoading()

  const handleClick = () => {
    if (isBusy() || isDeleting()) return
    if (isServerMode()) {
      navigate.navigate(`/library/server/${props.instanceId}`)
    } else {
      globalStore.markInstanceAsSeen(props.instanceId)
      setClickedInstanceId(`favorites-${props.instanceId}`)
      requestAnimationFrame(() => {
        navigate.navigate(`/library/${props.instanceId}`)
      })
    }
  }

  const handlePlay = (e?: MouseEvent) => {
    e?.stopPropagation()

    if (isServerMode()) {
      if (isServerRunning()) {
        stopServerMutation.mutate(props.instanceId)
      } else {
        startServerMutation.mutate(props.instanceId)
      }
      return
    }

    if (isQueued() || isPreparing()) return

    if (isRunning()) {
      killInstanceMutation.mutate(props.instanceId)
      return
    }

    if (
      globalStore.currentlySelectedAccount()?.status === "expired" ||
      globalStore.currentlySelectedAccount()?.status === "invalid"
    ) {
      modalsContext?.openModal(
        { name: "accountExpired" },
        { id: props.instanceId }
      )
      return
    }

    launchInstanceMutation.mutate({
      id: props.instanceId,
      skipMemoryCheck: false
    })
  }

  const handleDelete = () => {
    modalsContext?.openModal(
      { name: "confirmInstanceDeletion" },
      {
        id: props.instanceId,
        name: itemName() || "",
        isServer: isServerMode()
      }
    )
  }

  const handleToggleFavorite = () => {
    if (isServerMode()) {
      serverSetFavoriteMutation.mutate({
        id: props.instanceId,
        favorite: !server()?.favorite
      })
    } else {
      instanceSetFavoriteMutation.mutate({
        instance: props.instanceId,
        favorite: !instance()?.favorite
      })
    }
  }

  const handleSettings = () => {
    if (isServerMode()) {
      navigate.navigate(`/library/server/${props.instanceId}/settings`)
    } else {
      setClickedInstanceId(`favorites-${props.instanceId}`)
      requestAnimationFrame(() => {
        navigate.navigate(`/library/${props.instanceId}/settings`)
      })
    }
  }

  const handleEdit = () => {
    modalsContext?.openModal(
      { name: "instanceCreation" },
      {
        id: props.instanceId,
        modloader: validInstance()?.modloader,
        title: instance()?.name || "",
        mcVersion: validInstance()?.mc_version,
        modloaderVersion: validInstance()?.modloader_version,
        img: instanceImageUrl()
      }
    )
  }

  const handleDuplicate = () => {
    if (instance()?.status.status !== "invalid") {
      duplicateInstanceMutation.mutate({
        instance: props.instanceId,
        new_name: instance()?.name || ""
      })
    }
  }

  const handleOpenFolder = () => {
    openFolderMutation.mutate({
      instance_id: props.instanceId,
      folder: "Root"
    })
  }

  const staggerDelay = () => props.index * 40
  let wrapperRef: HTMLDivElement | undefined

  onMount(() => {
    if (props.isNewlyAdded && wrapperRef) {
      wrapperRef.animate(
        [
          { transform: "scale(0)", opacity: 0 },
          { transform: "scale(1.2)", opacity: 1, offset: 0.6 },
          { transform: "scale(1)", opacity: 1 }
        ],
        { duration: 300, easing: "ease-out", fill: "forwards" }
      )
    }
  })

  return (
    <Show when={itemExists()}>
      <div
        ref={wrapperRef}
        class="transition-all ease-spring"
        classList={{
          "opacity-0 scale-50 pointer-events-none": !props.expanded,
          "opacity-100 scale-100": props.expanded
        }}
        style={{
          "transition-duration": "250ms",
          "transition-delay": props.expanded ? `${staggerDelay()}ms` : "0ms"
        }}
      >
        <ContextMenu
          onOpenChange={(open) => {
            setIsMenuOpen(open)
            props.onMenuOpenChange(open)
          }}
        >
          <ContextMenuContent>
            <ContextMenuGroup>
              <ContextMenuGroupLabel>{itemName()}</ContextMenuGroupLabel>
              <ContextMenuSeparator />
              <ContextMenuItem
                class="flex items-center gap-2"
                onClick={() => handlePlay()}
                disabled={isBusy() || isDeleting()}
              >
                <div
                  class={`${isRunning() ? "i-hugeicons:stop" : "i-hugeicons:play"} h-4 w-4`}
                />
                {isRunning()
                  ? t("instances:_trn_stop")
                  : t("instances:_trn_action_play")}
              </ContextMenuItem>
              <Show when={!isServerMode()}>
                <ContextMenuItem
                  class="flex items-center gap-2"
                  onClick={handleEdit}
                  disabled={isLoading() || isDeleting()}
                >
                  <div class="i-hugeicons:pencil-edit-01 h-4 w-4" />
                  {t("instances:_trn_action_edit")}
                </ContextMenuItem>
              </Show>
              <ContextMenuItem
                class="flex items-center gap-2"
                onClick={handleSettings}
                disabled={isBusy() || isDeleting()}
              >
                <div class="i-hugeicons:settings-01 h-4 w-4" />
                {t("instances:_trn_action_settings")}
              </ContextMenuItem>
              <ContextMenuItem
                class="flex items-center gap-2"
                closeOnSelect={false}
                onClick={handleToggleFavorite}
              >
                <div
                  class="i-hugeicons:star h-4 w-4"
                  classList={{ "text-yellow-500": isFavorite() }}
                />
                {isFavorite()
                  ? t("instances:_trn_remove_favorite")
                  : t("instances:_trn_add_favorite")}
              </ContextMenuItem>
              <Show when={!isServerMode()}>
                <GdlFeatureContextMenuItem
                  icon={<div class="i-ri:share-line h-4 w-4" />}
                  onClick={() => {
                    modalsContext?.openModal(
                      { name: "shareInstance" },
                      { instanceId: props.instanceId }
                    )
                  }}
                  disabled={isLoading() || isDeleting()}
                >
                  {t("instances:_trn_instance_share.title")}
                </GdlFeatureContextMenuItem>
                <ContextMenuItem
                  class="flex items-center gap-2"
                  onClick={() => {
                    searchContext?.setSelectedInstanceId(props.instanceId)
                    setPayload({
                      target: "Curseforge",
                      save_path: undefined,
                      self_contained_addons_bundling: false,
                      filter: { entries: {} },
                      instance_id: props.instanceId
                    })
                    setExportStep(0)
                    setCheckedFiles([])
                    modalsContext?.openModal(
                      { name: "exportInstance" },
                      { instanceId: props.instanceId }
                    )
                  }}
                  disabled={isLoading() || isDeleting()}
                >
                  <div class="i-hugeicons:file-export h-4 w-4" />
                  {t("instances:_trn_export_instance")}
                </ContextMenuItem>
                <ContextMenuSeparator />
                <ContextMenuSub>
                  <ContextMenuSubTrigger>
                    {t("instances:_trn_more_options")}
                  </ContextMenuSubTrigger>
                  <ContextMenuPortal>
                    <ContextMenuSubContent>
                      <ContextMenuItem
                        class="flex items-center gap-2"
                        onClick={handleOpenFolder}
                      >
                        <div class="i-hugeicons:folder-open h-4 w-4" />
                        {t("instances:_trn_action_open_folder")}
                      </ContextMenuItem>
                      <ContextMenuItem
                        class="flex items-center gap-2"
                        onClick={() => {
                          navigate.navigate(`/library/${props.instanceId}/logs`)
                        }}
                      >
                        <div class="i-hugeicons:file-script h-4 w-4" />
                        {t("instances:_trn_view_logs")}
                      </ContextMenuItem>
                      <ContextMenuItem
                        class="flex items-center gap-2"
                        onClick={() => {
                          navigate.navigate(
                            `/library/${props.instanceId}/addons`
                          )
                        }}
                      >
                        <div class="i-hugeicons:puzzle h-4 w-4" />
                        {t("instances:_trn_view_mods")}
                      </ContextMenuItem>
                      <Show when={instance()?.status.status !== "invalid"}>
                        <ContextMenuItem
                          class="flex items-center gap-2"
                          onClick={handleDuplicate}
                          disabled={isLoading() || isDeleting()}
                        >
                          <div class="i-hugeicons:copy-01 h-4 w-4" />
                          {t("instances:_trn_action_duplicate")}
                        </ContextMenuItem>
                      </Show>
                    </ContextMenuSubContent>
                  </ContextMenuPortal>
                </ContextMenuSub>
              </Show>
              <ContextMenuSeparator />
              <ContextMenuItem
                class="flex items-center gap-2"
                onClick={handleDelete}
                disabled={isBusy() || isDeleting()}
              >
                <div class="i-hugeicons:delete-02 h-4 w-4" />
                {t("instances:_trn_action_delete")}
              </ContextMenuItem>
            </ContextMenuGroup>
          </ContextMenuContent>
          <ContextMenuTrigger>
            <Tooltip placement="top">
              <TooltipTrigger as="div">
                <div
                  class="group relative h-12 w-12 shrink-0 cursor-pointer rounded-full transition-all duration-200 hover:scale-110"
                  classList={{
                    "opacity-50": isDeleting(),
                    "ring-2 ring-green-500": isRunning(),
                    "scale-110": isMenuOpen(),
                    "opacity-0 pointer-events-none": isBeingDragged()
                  }}
                  onClick={handleClick}
                  onPointerDown={handlePointerDown}
                  onMouseEnter={() => {
                    if (!isServerMode()) {
                      globalStore.markInstanceAsSeen(props.instanceId)
                    }
                  }}
                >
                  {/* Avatar image */}
                  <Show
                    when={!isServerMode()}
                    fallback={
                      <div class="h-full w-full rounded-full bg-darkSlate-600 flex items-center justify-center">
                        <div class="i-hugeicons:server text-lightSlate-400 h-6 w-6" />
                      </div>
                    }
                  >
                    <img
                      src={instanceImageUrl() || DefaultImg}
                      alt={instance()?.name || ""}
                      class="h-full w-full rounded-full object-cover"
                    />
                  </Show>

                  {/* Running pulse dot */}
                  <Show when={isRunning()}>
                    <div class="absolute -top-0.5 -right-0.5">
                      <div class="h-3 w-3 rounded-full bg-green-500 animate-pulse shadow-lg shadow-green-500/50" />
                    </div>
                  </Show>

                  {/* Launching / busy spinner overlay */}
                  <Show when={isBusy() && !isDeleting()}>
                    <div class="absolute inset-0 flex items-center justify-center rounded-full bg-black/60 z-10">
                      <Spinner class="h-6 w-6" />
                    </div>
                  </Show>

                  {/* Play button overlay on hover */}
                  <div
                    class="absolute inset-0 flex items-center justify-center rounded-full opacity-0 transition-all duration-200"
                    classList={{
                      "group-hover:opacity-100":
                        !isBusy() && !isDeleting() && !props.isDragActive,
                      "!opacity-100":
                        isMenuOpen() && !isBusy() && !isDeleting(),
                      "bg-red-500/80": isRunning(),
                      "bg-primary-500/80": !isRunning()
                    }}
                    onClick={(e) => handlePlay(e)}
                  >
                    <div
                      class={`${isRunning() ? "i-hugeicons:stop" : "i-hugeicons:play"} text-white h-5 w-5`}
                    />
                  </div>
                </div>
              </TooltipTrigger>
              <TooltipContent class="min-w-48 px-3 py-2.5">
                <div class="text-sm font-bold text-white truncate max-w-48">
                  {itemName()}
                </div>
                <Show
                  when={!isServerMode()}
                  fallback={
                    <div class="flex flex-col gap-0.5 mt-1.5 text-[11px] text-lightSlate-600">
                      <div class="flex items-center gap-1.5">
                        <span>MC {server()?.gameVersion}</span>
                        <span>·</span>
                        <span>:{server()?.port}</span>
                      </div>
                      <div class="capitalize">{serverStatus()}</div>
                    </div>
                  }
                >
                  <div class="flex flex-col gap-0.5 mt-1.5 text-[11px] text-lightSlate-600">
                    <Show when={mcVersion() || modloader()}>
                      <div class="flex items-center gap-1.5">
                        <Show when={mcVersion()}>
                          <span>MC {mcVersion()}</span>
                        </Show>
                        <Show when={modloader() && mcVersion()}>
                          <span>·</span>
                        </Show>
                        <Show when={modloader()}>
                          <div class="flex items-center gap-1">
                            <img
                              class="h-3 w-3"
                              src={getModloaderIcon(modloader()!)}
                              alt=""
                            />
                            <span class="capitalize">{modloader()}</span>
                            <Show when={modloaderVersion()}>
                              <span>{modloaderVersion()}</span>
                            </Show>
                          </div>
                        </Show>
                      </div>
                    </Show>
                    <div>{lastPlayed()}</div>
                    <div>{playtime()}</div>
                  </div>
                </Show>
                <Show when={isRunning()}>
                  <div class="flex items-center gap-1 mt-1.5 text-[11px] text-green-400 font-medium">
                    <div class="h-1.5 w-1.5 rounded-full bg-green-500 animate-pulse" />
                    <Trans key="instances:_trn_server_status_running" />
                  </div>
                </Show>
              </TooltipContent>
            </Tooltip>
          </ContextMenuTrigger>
        </ContextMenu>
      </div>
    </Show>
  )
}
