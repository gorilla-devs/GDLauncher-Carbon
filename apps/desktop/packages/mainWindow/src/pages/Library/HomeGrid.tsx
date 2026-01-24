import {
  Button,
  Collapsable,
  ContextMenu,
  ContextMenuContent,
  ContextMenuGroup,
  ContextMenuGroupLabel,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuPortal,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
  Input,
  Skeleton
} from "@gd/ui"
import { useDragSelect } from "@/hooks/useDragSelect"
import {
  For,
  Match,
  Show,
  Switch,
  createEffect,
  createMemo,
  createSignal,
  on,
  onCleanup,
  onMount
} from "solid-js"
import { Trans, useTransContext } from "@gd/i18n"
import InstanceTile from "@/components/InstanceTile"
import UnstableCard from "@/components/UnstableCard"
import { PlaceholderGorilla } from "@/components/PlaceholderGorilla"
import { SelectionActionBar } from "./components/SelectionActionBar"
import {
  InstancesGroupBy,
  InstancesSortBy,
  ListInstance,
  ValidListInstance
} from "@gd/core_module/bindings"
import { rspc } from "@/utils/rspcClient"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { useModal } from "@/managers/ModalsManager"
import { DragProvider, useDragContext, DropTarget } from "./DragContext"
import DragGhost from "@/components/DragGhost"
import FavoritesDropZone from "@/components/Library/FavoritesDropZone"
import FavoriteTile from "@/components/Library/FavoriteTile"
import GroupHeader from "@/components/Library/GroupHeader"
import FolderTile, {
  clickedFolderId,
  setClickedFolderId,
  setVisibleFolderIndices,
  injectFolderTransitionCSS,
  removeFolderTransitionCSS
} from "@/components/Library/FolderTile"
import ExpandedFolderContent from "@/components/Library/ExpandedFolderContent"
import LibraryItemTile from "@/components/Library/LibraryItemTile"
import "@/components/Library/folderTransitions.css"
import { createAutoAnimate } from "@formkit/auto-animate/solid"

const animatedInstanceIds = new Set<number>()
let initialAnimationComplete = false

// Animation tracking for iOS folder view (libraryItems: folders + ungrouped instances)
const animatedLibraryItemIds = new Set<string>() // "folder-{id}" or "instance-{id}"
const libraryInitialAnimationCompleteRef = { value: false }

// Stable object cache for <For> - preserves object identity across re-renders
type CachedLibraryItem =
  | { id: string; type: "instance"; data: ListInstance }
  | {
      id: string
      type: "folder"
      data: {
        id: number
        name: string
        libraryPosition: number | null
        instances: ListInstance[]
      }
    }
const libraryItemCache = new Map<string, CachedLibraryItem>()

// FLIP animation state - stores positions before reorder
const libraryItemPositions = new Map<string, DOMRect>()
// Store snapshot of item order to detect when data actually changes (async after mutation)
let libraryItemOrderSnapshot: string[] | null = null
// Safety timeout to re-enable auto-animate if mutation fails
let flipAnimationTimeoutId: ReturnType<typeof setTimeout> | null = null

// End of group drop zone component
const EndOfGroupDropZone = (props: {
  groupId: number
  instanceCount: number
}) => {
  const dragContext = useDragContext()
  let ref: HTMLDivElement | undefined

  const isOver = () => {
    const target = dragContext.dropTarget()
    return target?.type === "endOfGroup" && target.groupId === props.groupId
  }

  // Register drop zone
  createEffect(() => {
    if (
      dragContext.isDragging() &&
      dragContext.dragType() === "instance" &&
      ref
    ) {
      const rect = ref.getBoundingClientRect()
      dragContext.registerDropZone({
        id: `end-of-group-${props.groupId}`,
        rect,
        target: { type: "endOfGroup", groupId: props.groupId }
      })
    } else {
      dragContext.unregisterDropZone(`end-of-group-${props.groupId}`)
    }
  })

  onCleanup(() => {
    dragContext.unregisterDropZone(`end-of-group-${props.groupId}`)
  })

  return (
    <div
      ref={ref}
      class="relative flex items-center justify-center min-w-16 h-24 rounded-lg transition-all duration-200"
      classList={{
        "border-2 border-dashed border-darkSlate-500": !isOver(),
        "border-2 border-solid border-primary-500 bg-primary-500/10": isOver()
      }}
    >
      <Show when={isOver()}>
        <div class="absolute -left-2 top-0 bottom-0 w-1 bg-primary-500 rounded-full z-50">
          <div class="absolute -top-1 left-1/2 -translate-x-1/2 w-2 h-2 rounded-full bg-primary-500" />
          <div class="absolute -bottom-1 left-1/2 -translate-x-1/2 w-2 h-2 rounded-full bg-primary-500" />
        </div>
      </Show>
      <div
        class="i-hugeicons:plus text-lg transition-colors"
        classList={{
          "text-darkSlate-500": !isOver(),
          "text-primary-500": isOver()
        }}
      />
    </div>
  )
}

// End of groups drop zone for folder reordering
const EndOfGroupsDropZone = () => {
  const dragContext = useDragContext()
  let ref: HTMLDivElement | undefined

  const isOver = () => {
    const target = dragContext.dropTarget()
    return target?.type === "endOfGroups"
  }

  // Register drop zone
  createEffect(() => {
    if (dragContext.isDragging() && dragContext.dragType() === "group" && ref) {
      const rect = ref.getBoundingClientRect()
      dragContext.registerDropZone({
        id: "end-of-groups",
        rect,
        target: { type: "endOfGroups" }
      })
    } else {
      dragContext.unregisterDropZone("end-of-groups")
    }
  })

  onCleanup(() => {
    dragContext.unregisterDropZone("end-of-groups")
  })

  return (
    <div
      ref={ref}
      class="relative flex items-center justify-center min-w-16 h-24 rounded-lg transition-all duration-200"
      classList={{
        "border-2 border-dashed border-darkSlate-500": !isOver(),
        "border-2 border-solid border-primary-500 bg-primary-500/10": isOver()
      }}
    >
      <Show when={isOver()}>
        <div class="absolute -left-2 top-0 bottom-0 w-1 bg-primary-500 rounded-full z-50">
          <div class="absolute -top-1 left-1/2 -translate-x-1/2 w-2 h-2 rounded-full bg-primary-500" />
          <div class="absolute -bottom-1 left-1/2 -translate-x-1/2 w-2 h-2 rounded-full bg-primary-500" />
        </div>
      </Show>
      <div
        class="i-hugeicons:plus text-lg transition-colors"
        classList={{
          "text-darkSlate-500": !isOver(),
          "text-primary-500": isOver()
        }}
      />
    </div>
  )
}

// Wrapper component that provides DragProvider
const HomeGrid = () => {
  return (
    <DragProvider>
      <HomeGridInner />
    </DragProvider>
  )
}

const HomeGridInner = () => {
  const [t] = useTransContext()
  const dragContext = useDragContext()

  const [filter, setFilter] = createSignal("")

  const globalStore = useGlobalStore()

  // Auto-animate refs for grid containers (respects reduced motion)
  const [favoritesGridRef, setFavoritesGridEnabled] = createAutoAnimate({
    duration: 200,
    easing: "ease-out"
  })
  const [mainGridRef, setMainGridEnabled] = createAutoAnimate({
    duration: 200,
    easing: "ease-out"
  })

  // Disable auto-animate when reduced motion is enabled
  createEffect(() => {
    const reducedMotion = globalStore.settings.data?.reducedMotion ?? false
    setFavoritesGridEnabled(!reducedMotion)
    setMainGridEnabled(!reducedMotion)
  })

  // Helper to cleanup FLIP animation state (called on success or timeout)
  const cleanupFlipAnimationState = () => {
    if (flipAnimationTimeoutId) {
      clearTimeout(flipAnimationTimeoutId)
      flipAnimationTimeoutId = null
    }
    libraryItemOrderSnapshot = null
    libraryItemPositions.clear()
    // Re-enable auto-animate (respects reduced motion)
    const reducedMotion = globalStore.settings.data?.reducedMotion ?? false
    setFavoritesGridEnabled(!reducedMotion)
    setMainGridEnabled(!reducedMotion)
  }

  const modals = useModal()

  const [instancesTileSize, setInstancesTileSize] = createSignal(2)

  // Multi-selection state
  const [selectedIds, setSelectedIds] = createSignal<Set<number>>(new Set())

  const toggleSelection = (id: number) => {
    setSelectedIds((prev) => {
      const newSet = new Set(prev)
      if (newSet.has(id)) {
        newSet.delete(id)
      } else {
        newSet.add(id)
      }
      return newSet
    })
  }

  const clearSelection = () => {
    setSelectedIds(new Set<number>())
  }

  const isSelected = (id: number) => selectedIds().has(id)

  // Store refs for all instance tiles (for drag selection)
  const tileRefs = new Map<number, HTMLDivElement>()

  // Store refs for ALL library items (for FLIP animation)
  const libraryItemRefs = new Map<string, HTMLDivElement>()

  // Capture positions of all library items (called before reorder)
  const captureLibraryItemPositions = () => {
    // Don't clear - update positions in place
    // Preserves positions for refs that may not be in libraryItemRefs yet
    libraryItemRefs.forEach((el, key) => {
      if (el.isConnected) {
        libraryItemPositions.set(key, el.getBoundingClientRect())
      }
    })
  }

  // Run FLIP animation after reorder
  const runFlipAnimation = () => {
    // Clear the safety timeout since animation is running
    if (flipAnimationTimeoutId) {
      clearTimeout(flipAnimationTimeoutId)
      flipAnimationTimeoutId = null
    }

    if (globalStore.settings.data?.reducedMotion) {
      libraryItemPositions.clear()
      return
    }

    // Iterate over captured positions, not refs
    // This ensures items that were recreated during DOM reconciliation still animate
    libraryItemPositions.forEach((oldRect, key) => {
      const el = libraryItemRefs.get(key)
      if (!el || !el.isConnected) return

      const newRect = el.getBoundingClientRect()
      const dx = oldRect.left - newRect.left
      const dy = oldRect.top - newRect.top

      if (dx === 0 && dy === 0) return // No movement

      // Apply FLIP animation
      el.animate([
        { transform: `translate(${dx}px, ${dy}px)` },
        { transform: 'translate(0, 0)' }
      ], {
        duration: 300,
        easing: 'ease-out'
      })
    })

    // Clear after animation to ensure fresh capture next time
    libraryItemPositions.clear()

    // Re-enable auto-animate after FLIP completes (respects reduced motion)
    const reducedMotion = globalStore.settings.data?.reducedMotion ?? false
    setFavoritesGridEnabled(!reducedMotion)
    setMainGridEnabled(!reducedMotion)
  }

  // Function to get bounding rects for all tiles
  const getItemRects = (): Map<number, DOMRect> => {
    const rects = new Map<number, DOMRect>()
    tileRefs.forEach((el, id) => {
      const rect = el.getBoundingClientRect()
      rects.set(id, rect)
    })
    return rects
  }

  // Drag selection hook
  const dragSelect = useDragSelect({
    containerRef: () => undefined,
    getItemRects,
    onSelectionChange: (selectedIds) => {
      setSelectedIds(new Set(selectedIds))
    }
  })

  // Check if a mouse event should be ignored (on tiles or interactive elements)
  const shouldIgnoreClick = (e: MouseEvent): boolean => {
    const target = e.target as HTMLElement
    // Ignore clicks on tiles, inputs, buttons, and interactive elements
    // Also ignore if drag and drop is active or folder is open
    return (
      !dragContext.dragSelectEnabled() ||
      openFolderId() !== null ||
      target.closest("[data-instance-tile]") !== null ||
      target.closest("[data-folder-tile]") !== null ||
      target.closest("input") !== null ||
      target.closest("button") !== null ||
      target.closest("[data-kb-menu]") !== null ||
      target.closest("[role='menu']") !== null
    )
  }

  // Escape key handler to clear selection and cancel drag
  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      if (dragContext.isDragging()) {
        dragContext.cancelDrag()
      } else if (selectedIds().size > 0) {
        clearSelection()
      }
    }
  }

  onMount(() => {
    document.addEventListener("keydown", handleKeyDown)
  })

  onCleanup(() => {
    document.removeEventListener("keydown", handleKeyDown)
  })

  const handleBatchDelete = () => {
    const selectedInstanceIds = Array.from(selectedIds())
    const selectedInstancesList = (globalStore.instances.data || []).filter(
      (instance) => selectedInstanceIds.includes(instance.id)
    )

    modals?.openModal(
      {
        name: "confirmBatchInstanceDeletion"
      },
      {
        instances: selectedInstancesList,
        onComplete: clearSelection
      }
    )
  }

  createEffect(() => {
    setInstancesTileSize(globalStore.settings.data?.instancesTileSize!)
  })

  const settingsMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setSettings"]
  }))

  // Drag and drop mutations
  const moveInstanceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.moveInstance"]
  }))

  const setFavoriteMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.setFavorite"]
  }))

  const moveGroupMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.moveGroup"]
  }))

  const createFolderFromInstancesMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.createFolderFromInstances"]
  }))

  const sortLibraryMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.sortLibrary"]
  }))

  // State for open folder in iOS-style view
  const [openFolderId, setOpenFolderId] = createSignal<number | null>(null)

  const toggleFolder = async (folderId: number) => {
    const shouldTransition =
      !globalStore.settings.data?.reducedMotion && document.startViewTransition

    if (shouldTransition) {
      // Get folder instance count for dynamic CSS injection
      const folder = libraryItems().find(
        (i) => i.type === "folder" && i.data.id === folderId
      )
      const instanceCount =
        folder?.type === "folder" ? folder.data.instances.length : 0

      // Only animate the 4 preview instances visible in the folder tile
      // Other instances will appear instantly (no view-transition-name = no animation)
      const maxVisibleOnOpen = Math.min(instanceCount, 4)
      const visibleIndices = Array.from({ length: maxVisibleOnOpen }, (_, i) => i)

      setVisibleFolderIndices(visibleIndices)
      injectFolderTransitionCSS(visibleIndices, "open")
      setClickedFolderId(folderId)

      // Wait for SolidJS to flush DOM updates before capturing old snapshot
      await new Promise((resolve) => queueMicrotask(resolve))

      const transition = document.startViewTransition(() => {
        setOpenFolderId((prev) => (prev === folderId ? null : folderId))
      })
      transition.finished.then(() => {
        setClickedFolderId(null)
        setVisibleFolderIndices([])
        removeFolderTransitionCSS()
      })
    } else {
      setOpenFolderId((prev) => (prev === folderId ? null : folderId))
    }
  }

  // Get the currently open folder data
  const getOpenFolder = createMemo(() => {
    const id = openFolderId()
    if (id === null) return null
    const item = libraryItems().find(
      (i) => i.type === "folder" && i.data.id === id
    )
    return item?.type === "folder" ? item.data : null
  })

  // Ref for favorites drop zone
  let favoritesDropZoneRef: HTMLDivElement | undefined

  // Function to handle drop
  const handleDrop = (
    target: DropTarget,
    draggedIds: number[],
    dragType: string
  ) => {
    if (!target || draggedIds.length === 0) return

    // Capture positions and order BEFORE mutation for FLIP animation
    captureLibraryItemPositions()
    // Store current order - animation runs when order actually changes (async)
    libraryItemOrderSnapshot = libraryItems().map(item => item.id)
    // Disable auto-animate to prevent conflict with manual FLIP
    setFavoritesGridEnabled(false)
    setMainGridEnabled(false)
    // Safety timeout: re-enable auto-animate if mutation fails or order doesn't change
    if (flipAnimationTimeoutId) clearTimeout(flipAnimationTimeoutId)
    flipAnimationTimeoutId = setTimeout(cleanupFlipAnimationState, 2000)

    if (dragType === "instance") {
      switch (target.type) {
        case "favorites": {
          // Toggle favorite status for all dragged instances
          const draggedInstances = (globalStore.instances.data || []).filter(
            (i) => draggedIds.includes(i.id)
          )
          const allAreFavorite = draggedInstances.every((i) => i.favorite)
          const newFavoriteStatus = !allAreFavorite

          for (const id of draggedIds) {
            setFavoriteMutation.mutate({
              instance: id,
              favorite: newFavoriteStatus
            })
          }
          break
        }
        case "beforeInstance": {
          // Move instances before target instance
          for (const id of draggedIds) {
            if (id !== target.instanceId) {
              moveInstanceMutation.mutate({
                instance: id,
                target: { BeforeInstance: target.instanceId }
              })
            }
          }
          break
        }
        case "endOfGroup": {
          // Move instances to end of group
          for (const id of draggedIds) {
            moveInstanceMutation.mutate({
              instance: id,
              target: { EndOfGroup: target.groupId }
            })
          }
          break
        }
        case "dropOnFolder": {
          // Move instances into folder (group)
          for (const id of draggedIds) {
            moveInstanceMutation.mutate({
              instance: id,
              target: { EndOfGroup: target.groupId }
            })
          }
          break
        }
        case "createFolder": {
          // Create new folder with the target instance and all dragged instances
          const allInstanceIds = [
            target.instanceId,
            ...draggedIds.filter((id) => id !== target.instanceId)
          ]
          createFolderFromInstancesMutation.mutate({
            instances: allInstanceIds
          })
          break
        }
        case "ungrouped": {
          // Move instances back to default group
          const _defaultGroupId = defaultGroupId()
          if (_defaultGroupId) {
            for (const id of draggedIds) {
              moveInstanceMutation.mutate({
                instance: id,
                target: { EndOfGroup: _defaultGroupId }
              })
            }
          }
          break
        }
        case "beforeInstanceAtFolder": {
          // Move instances to default group, positioned before the folder
          for (const id of draggedIds) {
            moveInstanceMutation.mutate({
              instance: id,
              target: { BeforeGroup: target.folderId }
            })
          }
          break
        }
        default:
          break
      }

      // Clear selection after drop
      clearSelection()
    } else if (dragType === "group") {
      // Handle group reordering
      const groupId = draggedIds[0]

      switch (target.type) {
        case "beforeGroup": {
          // Move group before target group
          if (groupId !== target.groupId) {
            moveGroupMutation.mutate({
              group: groupId,
              target: { BeforeGroup: target.groupId }
            })
          }
          break
        }
        case "beforeGroupAtInstance": {
          // Move group before an ungrouped instance
          moveGroupMutation.mutate({
            group: groupId,
            target: { BeforeInstance: target.beforeInstanceId }
          })
          break
        }
        case "endOfGroups":
        case "endOfLibrary": {
          // Move group to end of library
          moveGroupMutation.mutate({
            group: groupId,
            target: "EndOfLibrary"
          })
          break
        }
        default:
          break
      }
    }
  }

  // Register drop handler
  onMount(() => {
    dragContext.setOnDrop(handleDrop)
  })

  onCleanup(() => {
    dragContext.setOnDrop(null)
  })

  let inputRef: HTMLInputElement | undefined

  type Groups = Record<
    string | number,
    {
      id: string | number | null
      name: string
      instances: ListInstance[]
    }
  >

  const filteredGroups = createMemo(() => {
    const _groups: Groups = {}

    const nameFilter = filter().replaceAll(" ", "").toLowerCase()

    if (globalStore.settings.data?.instancesGroupBy === "group") {
      _groups.favorites = {
        id: -1,
        name: t("instances:_trn_favorites"),
        instances: []
      }

      // Pre-populate all groups from the database so empty groups are shown
      for (const group of globalStore.instanceGroups.data || []) {
        const groupName =
          group.name === "localize➽default"
            ? t("general:_trn_default")
            : group.name
        _groups[groupName] = {
          id: group.id,
          name: groupName,
          instances: []
        }
      }
    }

    for (const instance of globalStore.instances.data || []) {
      let groupId = null
      let groupName = null

      const validInstance =
        instance.status.status === "valid" ? instance.status.value : undefined

      if (globalStore.settings.data?.instancesGroupBy === "group") {
        const _groupName = globalStore.instanceGroups.data?.find(
          (group) => group.id === instance.group_id
        )?.name

        groupName =
          _groupName === "localize➽default"
            ? t("general:_trn_default")
            : _groupName
        groupId = instance.group_id
      } else if (
        globalStore.settings.data?.instancesGroupBy === "gameVersion"
      ) {
        if (instance.status.status === "valid") {
          groupName = validInstance?.mc_version
        }
      } else if (globalStore.settings.data?.instancesGroupBy === "modloader") {
        if (instance.status.status === "valid") {
          groupName = validInstance?.modloader || "vanilla"
        }
      } else if (
        globalStore.settings.data?.instancesGroupBy === "modplatform"
      ) {
        if (instance.status.status === "valid") {
          groupName = validInstance?.modpack?.type
        }
      }

      if (!groupName) {
        continue
      }

      if (!_groups[groupName]) {
        _groups[groupName] = {
          id: groupId,
          name: groupName,
          instances: []
        }
      }

      if (
        instance.name.toLowerCase().replaceAll(" ", "").includes(nameFilter)
      ) {
        if (
          globalStore.settings.data?.instancesGroupBy === "group" &&
          instance.favorite
        ) {
          _groups.favorites.instances.push(instance)
        }
        _groups[groupName].instances.push(instance)
      }
    }

    // sort groups
    for (const key in _groups) {
      _groups[key].instances.sort((a, b) => {
        let comparisonResult = 0 // Default comparison result

        if (globalStore.settings.data?.instancesSortBy === "manual") {
          comparisonResult = a.index - b.index
        } else if (globalStore.settings.data?.instancesSortBy === "name") {
          comparisonResult = a.name.localeCompare(b.name)
        } else if (
          globalStore.settings.data?.instancesSortBy === "mostPlayed"
        ) {
          comparisonResult = (a.seconds_played || 0) - (b.seconds_played || 0)
        } else if (
          globalStore.settings.data?.instancesSortBy === "lastPlayed"
        ) {
          const aLastPlayed = a.last_played ? Date.parse(a.last_played) : 0
          const bLastPlayed = b.last_played ? Date.parse(b.last_played) : 0
          comparisonResult = aLastPlayed - bLastPlayed
        } else if (
          globalStore.settings.data?.instancesSortBy === "lastUpdated"
        ) {
          const aLastUpdated = a.date_updated ? Date.parse(a.date_updated) : 0
          const bLastUpdated = b.date_updated ? Date.parse(b.date_updated) : 0
          comparisonResult = aLastUpdated - bLastUpdated
        } else if (
          globalStore.settings.data?.instancesSortBy === "gameVersion"
        ) {
          comparisonResult = (
            (a.status.value as ValidListInstance).mc_version || ""
          ).localeCompare(
            (b.status.value as ValidListInstance).mc_version || "",
            undefined,
            { numeric: true, sensitivity: "base" }
          )
        } else if (globalStore.settings.data?.instancesSortBy === "created") {
          const aCreated = a.date_created ? Date.parse(a.date_created) : 0
          const bCreated = b.date_created ? Date.parse(b.date_created) : 0
          comparisonResult = aCreated - bCreated
        }

        // If descending order is selected, invert the comparison result
        if (!globalStore.settings.data?.instancesSortByAsc) {
          comparisonResult = -comparisonResult
        }

        // Use name as a secondary sort criteria to ensure consistent order where primary criteria are equal
        return comparisonResult || a.name.localeCompare(b.name)
      })
    }

    return _groups
  })

  const iterableFilteredGroups = createMemo(() => {
    const iterable = Object.values(filteredGroups())

    if (globalStore.settings.data?.instancesGroupBy === "gameVersion") {
      iterable.sort((a, b) => {
        if (globalStore.settings.data?.instancesGroupByAsc) {
          return a.name.localeCompare(b.name, undefined, {
            numeric: true,
            sensitivity: "base"
          })
        } else {
          return b.name.localeCompare(a.name, undefined, {
            numeric: true,
            sensitivity: "base"
          })
        }
      })
    } else {
      iterable.sort((a, b) => {
        if (a.name === t("instances:_trn_favorites")) {
          return -1
        }

        if (b.name === t("instances:_trn_favorites")) {
          return 1
        }

        if (globalStore.settings.data?.instancesGroupByAsc) {
          return a.name.localeCompare(b.name)
        } else {
          return b.name.localeCompare(a.name)
        }
      })
    }

    return iterable
  })

  // Directly use the memo result instead of a separate store
  // This ensures sorting changes are immediately reflected
  const displayedGroups = createMemo(() => iterableFilteredGroups())

  // iOS-style folder view data structures
  type LibraryItem =
    | { id: string; type: "instance"; data: ListInstance }
    | {
        id: string
        type: "folder"
        data: { id: number; name: string; libraryPosition: number | null; instances: ListInstance[] }
      }

  // Get the default group ID from query
  const defaultGroupQuery = rspc.createQuery(() => ({
    queryKey: ["instance.getDefaultGroup"]
  }))

  const defaultGroupId = createMemo(() => defaultGroupQuery.data ?? null)

  // Favorite instance IDs (static row at top)
  // Returns primitive IDs - SolidJS <For> uses value equality for primitives
  // FavoriteTile looks up instance from globalStore, creating signal dependency for reactivity
  const favoriteInstanceIds = createMemo((): number[] => {
    const nameFilter = filter().replaceAll(" ", "").toLowerCase()
    return (globalStore.instances.data || [])
      .filter(
        (i) =>
          i.favorite &&
          i.name.toLowerCase().replaceAll(" ", "").includes(nameFilter)
      )
      .map((i) => i.id as unknown as number)
  })

  // Library items: ungrouped instances + folder tiles (when in folder view mode)
  // Uses stable object cache to preserve identity for <For> - enables auto-animate reorder detection
  const libraryItems = createMemo((): LibraryItem[] => {
    const items: LibraryItem[] = []
    const nameFilter = filter().replaceAll(" ", "").toLowerCase()
    const _defaultGroupId = defaultGroupId()

    if (!_defaultGroupId) return items

    // Get all groups and instances from the store
    const groups = globalStore.instanceGroups.data || []
    const allInstances = globalStore.instances.data || []

    // DEBUG: Trace group data flow
    console.log("[DEBUG] Total groups from store:", groups.length, groups.map(g => ({ id: g.id, name: g.name })))

    // Group instances by group_id
    const instancesByGroup = new Map<number, ListInstance[]>()
    for (const instance of allInstances) {
      const list = instancesByGroup.get(instance.group_id) || []
      list.push(instance)
      instancesByGroup.set(instance.group_id, list)
    }

    // Track which keys we see this render (for cache cleanup)
    const seenKeys = new Set<string>()

    for (const group of groups) {
      const groupInstances = instancesByGroup.get(group.id) || []
      // Filter instances by name
      const filteredInstances = groupInstances.filter((inst: ListInstance) =>
        inst.name.toLowerCase().replaceAll(" ", "").includes(nameFilter)
      )

      if (group.id === _defaultGroupId) {
        // Default group instances become ungrouped items (excluding favorites)
        for (const instance of filteredInstances) {
          if (!instance.favorite) {
            const key = `instance-${instance.id}`
            seenKeys.add(key)

            // Get or create cached wrapper - PRESERVES OBJECT IDENTITY
            let cached = libraryItemCache.get(key)
            if (!cached || cached.type !== "instance") {
              cached = { id: key, type: "instance", data: instance }
              libraryItemCache.set(key, cached)
            } else {
              // Update data in existing object (keeps same reference)
              cached.data = instance
            }
            items.push(cached)
          }
        }
      } else {
        // Other groups become folder items (show even if empty for filter, but only if has filtered results or is empty before filter)
        const nonFavoriteInstances = groupInstances.filter(
          (inst: ListInstance) => !inst.favorite
        )
        const hasAnyInstances = nonFavoriteInstances.length > 0
        const filteredNonFavorites = filteredInstances.filter(
          (inst: ListInstance) => !inst.favorite
        )
        const hasFilteredInstances = filteredNonFavorites.length > 0

        // Show folder if it has no instances (empty folder) or has filtered instances
        if (!hasAnyInstances || hasFilteredInstances) {
          const key = `folder-${group.id}`
          seenKeys.add(key)

          const folderData = {
            id: group.id,
            name:
              group.name === "localize➽default"
                ? t("general:_trn_default")
                : group.name,
            libraryPosition: group.library_position,
            instances: filteredNonFavorites
          }

          // Get or create cached wrapper - PRESERVES OBJECT IDENTITY
          let cached = libraryItemCache.get(key)
          if (!cached || cached.type !== "folder") {
            cached = { id: key, type: "folder", data: folderData }
            libraryItemCache.set(key, cached)
          } else {
            // Update data in existing object (keeps same reference)
            ;(cached as { id: string; type: "folder"; data: typeof folderData }).data = folderData
          }
          items.push(cached)
        }
      }
    }

    // DEBUG: Log how many folders were added
    console.log("[DEBUG] Folders added to items:", items.filter(i => i.type === "folder").length)

    // Clean up cache entries for items no longer present
    for (const key of libraryItemCache.keys()) {
      if (!seenKeys.has(key)) {
        libraryItemCache.delete(key)
      }
    }

    // Sort items by libraryPosition for interleaving
    // Items with libraryPosition are sorted by that value
    // Items without (shouldn't happen at root level) fall back to index
    items.sort((a, b) => {
      const getKey = (item: LibraryItem) => {
        if (item.type === "instance") {
          // Use libraryPosition if available (for ungrouped instances), fallback to index
          return item.data.library_position ?? item.data.index
        } else {
          // For folders, use libraryPosition if available, fallback to a large number
          return item.data.libraryPosition ?? 10000
        }
      }
      return getKey(a) - getKey(b)
    })

    return items
  })

  // FLIP animation effect - runs after libraryItems changes
  createEffect(() => {
    // Track libraryItems to trigger on change
    const items = libraryItems()

    // Only animate if we have a snapshot and the order has actually changed
    if (libraryItemOrderSnapshot && items.length > 0) {
      const currentOrder = items.map(item => item.id)
      const orderChanged = libraryItemOrderSnapshot.length !== currentOrder.length ||
        libraryItemOrderSnapshot.some((id, i) => id !== currentOrder[i])

      if (orderChanged) {
        // Clear snapshot before animation
        libraryItemOrderSnapshot = null
        // Double RAF ensures DOM reconciliation completes before measuring positions
        // Auto-animate is disabled during FLIP, so no conflict occurs
        requestAnimationFrame(() => {
          requestAnimationFrame(() => {
            runFlipAnimation()
          })
        })
      }
    }
  })

  // Check if we're in iOS-style folder view mode
  const isIosFolderView = createMemo(
    () => globalStore.settings.data?.instancesGroupBy === "group"
  )

  // Reset animation state when switching view modes (but NOT on initial mount)
  // Using on() with defer: true skips the first run, so animation state
  // persists across navigation and only resets when view mode actually changes
  createEffect(
    on(isIosFolderView, () => {
      // Reset the library animation tracking when switching views
      animatedLibraryItemIds.clear()
      libraryInitialAnimationCompleteRef.value = false

      // Also reset collapsible view animation tracking
      animatedInstanceIds.clear()
      initialAnimationComplete = false
    }, { defer: true })
  )

  const sortByOptions: {
    key: InstancesSortBy
    label: string
  }[] = [
    {
      key: "manual",
      label: t("ui:_trn_manual")
    },
    {
      key: "name",
      label: t("ui:_trn_name")
    },
    {
      key: "mostPlayed",
      label: t("ui:_trn_most_played")
    },
    {
      key: "lastPlayed",
      label: t("ui:_trn_last_played")
    },
    {
      key: "lastUpdated",
      label: t("ui:_trn_last_updated")
    },
    {
      key: "gameVersion",
      label: t("ui:_trn_game_version")
    },
    {
      key: "created",
      label: t("ui:_trn_created")
    }
  ]

  const groupByOptions: {
    key: InstancesGroupBy
    label: string
  }[] = [
    {
      key: "group",
      label: t("ui:_trn_group")
    },
    {
      key: "gameVersion",
      label: t("ui:_trn_game_version")
    },
    {
      key: "modloader",
      label: t("ui:_trn_modloader")
    },
    {
      key: "modplatform",
      label: t("content:_trn_modplatform")
    }
  ]

  return (
    <div
      class="p-6"
      onMouseDown={(e) => {
        if (!shouldIgnoreClick(e)) {
          dragSelect.handlers.handleMouseDown(e)
        }
      }}
    >
      <UnstableCard />
      <Switch>
        <Match when={globalStore.instances.isLoading || (isIosFolderView() && !defaultGroupId())}>
          <div>
            <Skeleton.instances />
          </div>
        </Match>
        <Match
          when={
            globalStore.instances?.data?.length === 0 &&
            !globalStore.instances.isLoading
          }
        >
          <div class="mt-12 flex h-full w-full flex-col items-center justify-center gap-6">
            <PlaceholderGorilla
              size={14}
              variant="Welcoming Gorilla - Open Arms"
            />
            <p class="text-lightSlate-700 max-w-100 text-center">
              <Trans key="instances:_trn_no_instances_text" />
            </p>
          </div>
        </Match>
        <Match
          when={
            (globalStore.instances?.data?.length || 0) > 0 &&
            !globalStore.instances.isLoading
          }
        >
          <div>
            <div class="bg-darkSlate-800 z-5 sticky top-0 flex items-center gap-4 py-4">
              <Show
                when={
                  dragContext.isDragging() &&
                  dragContext.dragType() === "instance"
                }
                fallback={
                  <Input
                    ref={inputRef}
                    placeholder={t("search:_trn_search_instances")}
                    value={filter()}
                    class="w-full rounded-full"
                    onInput={(e) => setFilter(e.target.value)}
                    disabled={iterableFilteredGroups().length === 0}
                    icon={
                      <Switch>
                        <Match when={filter()}>
                          <div
                            class="hover:bg-white i-hugeicons:cancel-01"
                            onClick={() => {
                              setFilter("")
                            }}
                          />
                        </Match>
                        <Match when={!filter()}>
                          <div class="i-hugeicons:search-01" />
                        </Match>
                      </Switch>
                    }
                  />
                }
              >
                <div ref={favoritesDropZoneRef} class="w-full h-10">
                  <FavoritesDropZone
                    instances={globalStore.instances.data || []}
                    containerRef={favoritesDropZoneRef}
                  />
                </div>
              </Show>
              <DropdownMenu>
                <DropdownMenuTrigger>
                  <Button type="secondary" size="small">
                    <div class="i-hugeicons:filter h-4 w-4" />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent class="w-64">
                  <DropdownMenuLabel>
                    <div class="flex items-center justify-between gap-2">
                      <div>
                        <Trans key="content:_trn_platform" />
                      </div>
                      <div
                        class="text-lightSlate-900 hover:text-lightSlate-50 text-xs transition-colors duration-200 ease-[cubic-bezier(.4,0,.2,1)]"
                        onClick={() => {
                          // Reset all filter settings to defaults
                          settingsMutation.mutate({
                            instancesTileSize: { Set: 2 },
                            instancesSortBy: { Set: "created" },
                            instancesSortByAsc: { Set: false },
                            instancesGroupBy: { Set: "group" },
                            instancesGroupByAsc: { Set: true }
                          })
                          setInstancesTileSize(2)
                        }}
                      >
                        <Trans key="instances:_trn_reset_filters" />
                      </div>
                    </div>
                  </DropdownMenuLabel>
                  <DropdownMenuSeparator />

                  <div class="flex w-full flex-col">
                    <DropdownMenuSub>
                      <DropdownMenuSubTrigger class="w-full">
                        <div class="flex w-full items-center justify-between">
                          <Trans key="instances:_trn_instance_tile_size" />
                          <div class="flex items-center gap-2">
                            <span>{instancesTileSize()}</span>
                          </div>
                        </div>
                      </DropdownMenuSubTrigger>
                      <DropdownMenuPortal>
                        <DropdownMenuSubContent>
                          <DropdownMenuLabel>
                            <Trans key="ui:_trn_tile_size" />
                          </DropdownMenuLabel>
                          <DropdownMenuRadioGroup
                            value={instancesTileSize().toString()}
                          >
                            <For each={[1, 2, 3, 4, 5]}>
                              {(size) => (
                                <DropdownMenuRadioItem
                                  value={size.toString()}
                                  onSelect={() => {
                                    setInstancesTileSize(size)
                                    settingsMutation.mutate({
                                      instancesTileSize: {
                                        Set: size
                                      }
                                    })
                                  }}
                                >
                                  {size}
                                </DropdownMenuRadioItem>
                              )}
                            </For>
                          </DropdownMenuRadioGroup>
                        </DropdownMenuSubContent>
                      </DropdownMenuPortal>
                    </DropdownMenuSub>

                    <DropdownMenuSub>
                      <DropdownMenuSubTrigger class="w-full">
                        <div class="flex w-full items-center justify-between">
                          <Trans key="search:_trn_sort_by" />
                          <div class="flex items-center gap-2">
                            <span>
                              {sortByOptions.find(
                                (opt) =>
                                  opt.key ===
                                  globalStore.settings.data?.instancesSortBy
                              )?.label || "Name"}
                            </span>
                            {globalStore.settings.data?.instancesSortBy && (
                              <div
                                class={`ml-2 h-4 w-4 ${globalStore.settings.data?.instancesSortByAsc ? "i-hugeicons:arrange-by-letters-a-z" : "i-hugeicons:arrange-by-letters-z-a"}`}
                              />
                            )}
                          </div>
                        </div>
                      </DropdownMenuSubTrigger>
                      <DropdownMenuPortal>
                        <DropdownMenuSubContent>
                          <DropdownMenuLabel>
                            <Trans key="ui:_trn_sort_options" />
                          </DropdownMenuLabel>
                          <DropdownMenuRadioGroup
                            value={
                              globalStore.settings.data?.instancesSortBy || ""
                            }
                          >
                            <For each={sortByOptions}>
                              {(option) => (
                                <DropdownMenuRadioItem
                                  value={option.key}
                                  onSelect={() => {
                                    const currentOption =
                                      globalStore.settings.data?.instancesSortBy
                                    const currentDirection =
                                      globalStore.settings.data
                                        ?.instancesSortByAsc

                                    // If clicking the same option
                                    if (currentOption === option.key) {
                                      // Toggle direction
                                      settingsMutation.mutate({
                                        instancesSortByAsc: {
                                          Set: !currentDirection
                                        }
                                      })
                                    } else {
                                      // New option, set to ascending by default
                                      settingsMutation.mutate({
                                        instancesSortBy: {
                                          Set: option.key
                                        },
                                        instancesSortByAsc: {
                                          Set: true
                                        }
                                      })
                                    }
                                  }}
                                >
                                  <div class="flex w-full items-center justify-between">
                                    <span>{option.label}</span>
                                    {globalStore.settings.data
                                      ?.instancesSortBy === option.key && (
                                      <div
                                        class={`ml-4 h-4 w-4 ${globalStore.settings.data?.instancesSortByAsc ? "i-hugeicons:arrange-by-letters-a-z" : "i-hugeicons:arrange-by-letters-z-a"}`}
                                      />
                                    )}
                                  </div>
                                </DropdownMenuRadioItem>
                              )}
                            </For>
                          </DropdownMenuRadioGroup>
                        </DropdownMenuSubContent>
                      </DropdownMenuPortal>
                    </DropdownMenuSub>

                    <DropdownMenuSub>
                      <DropdownMenuSubTrigger class="w-full">
                        <div class="flex w-full items-center justify-between">
                          <Trans key="search:_trn_group_by" />
                          <div class="flex items-center gap-2">
                            <span>
                              {groupByOptions.find(
                                (opt) =>
                                  opt.key ===
                                  globalStore.settings.data?.instancesGroupBy
                              )?.label || "Group"}
                            </span>
                            {globalStore.settings.data?.instancesGroupBy && (
                              <div
                                class={`ml-2 h-4 w-4 ${globalStore.settings.data?.instancesGroupByAsc ? "i-hugeicons:arrange-by-letters-a-z" : "i-hugeicons:arrange-by-letters-z-a"}`}
                              />
                            )}
                          </div>
                        </div>
                      </DropdownMenuSubTrigger>
                      <DropdownMenuPortal>
                        <DropdownMenuSubContent>
                          <DropdownMenuLabel>
                            <Trans key="ui:_trn_group_options" />
                          </DropdownMenuLabel>
                          <DropdownMenuRadioGroup
                            value={
                              globalStore.settings.data?.instancesGroupBy || ""
                            }
                          >
                            <For each={groupByOptions}>
                              {(option) => (
                                <DropdownMenuRadioItem
                                  value={option.key}
                                  onSelect={() => {
                                    const currentOption =
                                      globalStore.settings.data
                                        ?.instancesGroupBy
                                    const currentDirection =
                                      globalStore.settings.data
                                        ?.instancesGroupByAsc

                                    // If clicking the same option
                                    if (currentOption === option.key) {
                                      // Toggle direction
                                      settingsMutation.mutate({
                                        instancesGroupByAsc: {
                                          Set: !currentDirection
                                        }
                                      })
                                    } else {
                                      // New option, set to ascending by default
                                      settingsMutation.mutate({
                                        instancesGroupBy: {
                                          Set: option.key
                                        },
                                        instancesGroupByAsc: {
                                          Set: true
                                        }
                                      })
                                    }
                                  }}
                                >
                                  <div class="flex w-full items-center justify-between">
                                    <span>{option.label}</span>
                                    {globalStore.settings.data
                                      ?.instancesGroupBy === option.key && (
                                      <div
                                        class={`ml-4 h-4 w-4 ${globalStore.settings.data?.instancesGroupByAsc ? "i-hugeicons:arrange-by-letters-a-z" : "i-hugeicons:arrange-by-letters-z-a"}`}
                                      />
                                    )}
                                  </div>
                                </DropdownMenuRadioItem>
                              )}
                            </For>
                          </DropdownMenuRadioGroup>
                        </DropdownMenuSubContent>
                      </DropdownMenuPortal>
                    </DropdownMenuSub>
                  </div>
                </DropdownMenuContent>
              </DropdownMenu>
              <Show
                when={globalStore.settings.data?.instancesGroupBy === "group"}
              >
                <DropdownMenu>
                  <DropdownMenuTrigger>
                    <Button
                      type="secondary"
                      size="small"
                      title={t("instances:_trn_rearrange")}
                    >
                      <div class="i-hugeicons:arrow-up-down h-4 w-4" />
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent>
                    <DropdownMenuLabel>
                      <Trans key="instances:_trn_rearrange" />
                    </DropdownMenuLabel>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem
                      onClick={() => sortLibraryMutation.mutate("name")}
                    >
                      <div class="flex items-center gap-2">
                        <div class="i-hugeicons:text h-4 w-4" />
                        <Trans key="ui:_trn_by_name" />
                      </div>
                    </DropdownMenuItem>
                    <DropdownMenuItem
                      onClick={() => sortLibraryMutation.mutate("lastPlayed")}
                    >
                      <div class="flex items-center gap-2">
                        <div class="i-hugeicons:clock-01 h-4 w-4" />
                        <Trans key="ui:_trn_by_last_played" />
                      </div>
                    </DropdownMenuItem>
                    <DropdownMenuItem
                      onClick={() => sortLibraryMutation.mutate("mostPlayed")}
                    >
                      <div class="flex items-center gap-2">
                        <div class="i-hugeicons:time-02 h-4 w-4" />
                        <Trans key="ui:_trn_by_most_played" />
                      </div>
                    </DropdownMenuItem>
                    <DropdownMenuItem
                      onClick={() => sortLibraryMutation.mutate("dateCreated")}
                    >
                      <div class="flex items-center gap-2">
                        <div class="i-hugeicons:calendar-add-01 h-4 w-4" />
                        <Trans key="ui:_trn_by_date_created" />
                      </div>
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
                <Button
                  type="secondary"
                  size="small"
                  onClick={() => {
                    modals?.openModal({ name: "createGroup" })
                  }}
                  title={t("instances:_trn_create_group")}
                >
                  <div class="i-hugeicons:folder-add h-4 w-4" />
                </Button>
              </Show>
            </div>
            <ContextMenu>
              <ContextMenuTrigger>
                <div class="mt-4" onClick={() => {
                  // Don't close folder if we're currently dragging or just finished a drag
                  if (!dragContext.isDragging() && !dragContext.justDropped()) {
                    setOpenFolderId(null)
                  }
                }}>
                  {/* iOS-style folder view when grouped by "group" */}
                  <Show when={isIosFolderView()}>
                    {/* Favorites Row - Large Prominent Cards (Max 3) */}
                    <Show when={favoriteInstanceIds().length > 0}>
                      <div class="mb-6">
                        {/* Header with star icon */}
                        <div class="flex items-center gap-2 mb-4">
                          <div class="i-ri:star-fill text-yellow-500 text-lg" />
                          <span class="text-base font-semibold text-lightSlate-300">
                            <Trans key="instances:_trn_favorites" /> ({Math.min(favoriteInstanceIds().length, 3)}/3)
                          </span>
                        </div>

                        {/* 3-column grid */}
                        <div
                          ref={favoritesGridRef}
                          class="grid grid-cols-3 gap-4"
                        >
                          <For each={favoriteInstanceIds().slice(0, 3)}>
                            {(instanceId) => (
                              <FavoriteTile
                                instanceId={instanceId}
                                isDragActive={dragContext.isDragging()}
                                preventClick={() => dragContext.justDropped()}
                              />
                            )}
                          </For>
                        </div>

                        {/* Subtle separator */}
                        <div class="border-t border-darkSlate-700 mt-6" />
                      </div>
                    </Show>

                    {/* Main Grid: Ungrouped instances + Folder tiles */}
                    <div
                      ref={mainGridRef}
                      class="relative flex flex-wrap gap-x-4 content-start"
                      classList={{
                        "gap-y-4": instancesTileSize() === 1,
                        "gap-y-6": instancesTileSize() === 2,
                        "gap-y-8": instancesTileSize() === 3,
                        "gap-y-10": instancesTileSize() === 4,
                        "gap-y-12": instancesTileSize() === 5
                      }}
                    >
                      <For each={libraryItems()}>
                        {(item, itemIndex) => (
                          <LibraryItemTile
                            item={item}
                            itemIndex={itemIndex}
                            instancesTileSize={instancesTileSize}
                            defaultGroupId={defaultGroupId}
                            openFolderId={openFolderId}
                            toggleFolder={toggleFolder}
                            isSelected={isSelected}
                            toggleSelection={toggleSelection}
                            selectedIds={selectedIds}
                            onDragStart={(type, ids, e) =>
                              dragContext.startDrag(type, ids, e)
                            }
                            justDropped={dragContext.justDropped}
                            tileRefs={tileRefs}
                            libraryItemRefs={libraryItemRefs}
                            animatedLibraryItemIds={animatedLibraryItemIds}
                            libraryInitialAnimationComplete={
                              libraryInitialAnimationCompleteRef
                            }
                            libraryItemsLength={() => libraryItems().length}
                          />
                        )}
                      </For>

                      {/* End of main grid drop zone for instances */}
                      <Show
                        when={
                          dragContext.isDragging() &&
                          dragContext.dragType() === "instance" &&
                          defaultGroupId()
                        }
                      >
                        <EndOfGroupDropZone
                          groupId={defaultGroupId()!}
                          instanceCount={
                            libraryItems().filter((i) => i.type === "instance")
                              .length
                          }
                        />
                      </Show>

                      {/* End of groups drop zone for folder reordering */}
                      <Show
                        when={
                          dragContext.isDragging() &&
                          dragContext.dragType() === "group"
                        }
                      >
                        <EndOfGroupsDropZone />
                      </Show>

                      {/* Folder overlay - rendered as last child of grid container */}
                      <Show when={getOpenFolder()}>
                        {(folder) => (
                          <ExpandedFolderContent
                            group={folder()}
                            onClose={() => setOpenFolderId(null)}
                            tileSize={instancesTileSize() as 1 | 2 | 3 | 4 | 5}
                            isDefaultGroup={false}
                            selectedIds={selectedIds()}
                            onToggleSelection={toggleSelection}
                            onSetSelection={(ids) => setSelectedIds(new Set(ids))}
                            onDragStart={(
                              instanceId,
                              isInstanceSelected,
                              e
                            ) => {
                              const ids = isInstanceSelected
                                ? Array.from(selectedIds())
                                : [instanceId]
                              dragContext.startDrag("instance", ids, e)
                            }}
                          />
                        )}
                      </Show>
                    </div>
                  </Show>

                  {/* Traditional collapsable group view (when not grouped by "group") */}
                  <Show when={!isIosFolderView()}>
                    <For each={displayedGroups()}>
                      {(group, i) => {
                        // Check if this is a database group (has numeric id) vs pseudo-group
                        const isDbGroup = () =>
                          typeof group.id === "number" &&
                          group.id > 0 &&
                          globalStore.settings.data?.instancesGroupBy ===
                            "group"

                        // Check if this is the default group
                        const isDefaultGroup = () => {
                          const dbGroup = globalStore.instanceGroups.data?.find(
                            (g) => g.id === group.id
                          )
                          return dbGroup?.name === "localize➽default"
                        }

                        // Show group if it has instances, OR if it's a database group (allow empty groups to be shown)
                        const shouldShowGroup = () =>
                          group.instances.length > 0 ||
                          (typeof group.id === "number" &&
                            group.id > 0 &&
                            globalStore.settings.data?.instancesGroupBy ===
                              "group")

                        return (
                          <Show when={shouldShowGroup()}>
                            <Collapsable
                              noPadding
                              title={
                                <>
                                  <span>{group.name}</span>
                                </>
                              }
                              size="standard"
                              customHeader={
                                isDbGroup()
                                  ? (toggle, isOpened) => (
                                      <GroupHeader
                                        groupId={group.id as number}
                                        name={group.name}
                                        isDefault={isDefaultGroup()}
                                        onToggleCollapse={toggle}
                                        isCollapsed={!isOpened()}
                                      />
                                    )
                                  : undefined
                              }
                            >
                              <div
                                class="mt-4 flex flex-wrap gap-x-4"
                                classList={{
                                  "gap-y-4": instancesTileSize() === 1,
                                  "gap-y-6": instancesTileSize() === 2,
                                  "gap-y-8": instancesTileSize() === 3,
                                  "gap-y-10": instancesTileSize() === 4,
                                  "gap-y-12": instancesTileSize() === 5
                                }}
                              >
                                <For each={group.instances}>
                                  {(instance, j) => {
                                    let ref: HTMLDivElement | undefined

                                    const instancesCountInPreviousGroups =
                                      displayedGroups()
                                        .slice(0, i())
                                        .reduce(
                                          (acc, group) =>
                                            acc + group.instances.length,
                                          0
                                        )

                                    const baseDelay = 100

                                    const groupDelay =
                                      i() * 60 +
                                      60 * instancesCountInPreviousGroups

                                    const instanceDelay = j() * 60

                                    const totalDelay =
                                      baseDelay + groupDelay + instanceDelay

                                    // Check if this instance is being dragged
                                    const isBeingDragged = () =>
                                      dragContext.isDragging() &&
                                      dragContext.dragType() === "instance" &&
                                      dragContext
                                        .draggedIds()
                                        .includes(instance.id)

                                    // Check if drop indicator should show before this instance
                                    const showDropIndicator = () => {
                                      const target = dragContext.dropTarget()
                                      return (
                                        dragContext.isDragging() &&
                                        dragContext.dragType() === "instance" &&
                                        target?.type === "beforeInstance" &&
                                        target.instanceId === instance.id
                                      )
                                    }

                                    // Register drop zone for this instance position
                                    createEffect(() => {
                                      if (
                                        dragContext.isDragging() &&
                                        dragContext.dragType() === "instance" &&
                                        ref &&
                                        typeof group.id === "number"
                                      ) {
                                        // Don't register drop zone for dragged instances
                                        if (
                                          dragContext
                                            .draggedIds()
                                            .includes(instance.id)
                                        ) {
                                          dragContext.unregisterDropZone(
                                            `before-instance-${instance.id}`
                                          )
                                          return
                                        }

                                        const rect = ref.getBoundingClientRect()
                                        // Create drop zone for the left third of the tile
                                        const dropRect = new DOMRect(
                                          rect.left - 8,
                                          rect.top,
                                          rect.width / 3 + 8,
                                          rect.height
                                        )

                                        dragContext.registerDropZone({
                                          id: `before-instance-${instance.id}`,
                                          rect: dropRect,
                                          target: {
                                            type: "beforeInstance",
                                            instanceId: instance.id,
                                            groupId: group.id as number
                                          }
                                        })
                                      } else {
                                        dragContext.unregisterDropZone(
                                          `before-instance-${instance.id}`
                                        )
                                      }
                                    })

                                    onMount(() => {
                                      // Only animate if this instance hasn't been animated yet AND initial animation is not complete
                                      const shouldAnimate =
                                        !animatedInstanceIds.has(instance.id) &&
                                        !initialAnimationComplete

                                      if (ref && shouldAnimate) {
                                        animatedInstanceIds.add(instance.id) // Mark as animated BEFORE animation starts
                                        ref.animate(
                                          [
                                            {
                                              opacity: 0
                                            },
                                            {
                                              opacity: 1
                                            }
                                          ],
                                          {
                                            duration: 250,
                                            delay: totalDelay,
                                            easing: "linear",
                                            fill: "forwards"
                                          }
                                        )
                                      }

                                      if (ref) {
                                        tileRefs.set(instance.id, ref)
                                      }

                                      // Mark initial animation complete after last instance
                                      if (
                                        i() === displayedGroups().length - 1 &&
                                        j() === group.instances.length - 1
                                      ) {
                                        requestAnimationFrame(() => {
                                          initialAnimationComplete = true
                                        })
                                      }
                                    })

                                    onCleanup(() => {
                                      tileRefs.delete(instance.id)
                                      dragContext.unregisterDropZone(
                                        `before-instance-${instance.id}`
                                      )
                                    })

                                    return (
                                      <div
                                        ref={ref}
                                        data-instance-tile
                                        class="relative"
                                        classList={{
                                          "opacity-0":
                                            !animatedInstanceIds.has(
                                              instance.id
                                            ) && !initialAnimationComplete
                                        }}
                                      >
                                        {/* Drop indicator before this instance */}
                                        <Show when={showDropIndicator()}>
                                          <div class="absolute -left-2.5 top-0 bottom-0 w-1.5 z-50 flex flex-col items-center">
                                            <div class="w-3 h-3 rounded-full bg-primary-500 -mt-1.5 shadow-lg shadow-primary-500/50" />
                                            <div class="flex-1 w-1 bg-gradient-to-b from-primary-500 via-primary-400 to-primary-500 rounded-full shadow-lg shadow-primary-500/40" />
                                            <div class="w-3 h-3 rounded-full bg-primary-500 -mb-1.5 shadow-lg shadow-primary-500/50" />
                                          </div>
                                        </Show>
                                        <InstanceTile
                                          instance={instance}
                                          identifier={`${group.id?.toString() || group.name} - ${instance.id}`}
                                          size={instancesTileSize() as any}
                                          isMultiSelected={isSelected(
                                            instance.id
                                          )}
                                          onToggleSelection={() =>
                                            toggleSelection(instance.id)
                                          }
                                          isDragging={isBeingDragged()}
                                          isDragActive={dragContext.isDragging()}
                                          groupId={
                                            typeof group.id === "number"
                                              ? group.id
                                              : undefined
                                          }
                                          onDragStart={(e) => {
                                            // If instance is selected, drag all selected
                                            // Otherwise just drag this one
                                            const ids = isSelected(instance.id)
                                              ? Array.from(selectedIds())
                                              : [instance.id]
                                            dragContext.startDrag(
                                              "instance",
                                              ids,
                                              e
                                            )
                                          }}
                                          preventClick={() =>
                                            dragContext.justDropped()
                                          }
                                        />
                                      </div>
                                    )
                                  }}
                                </For>
                                {/* End of group drop zone */}
                                <Show
                                  when={
                                    dragContext.isDragging() &&
                                    dragContext.dragType() === "instance" &&
                                    typeof group.id === "number" &&
                                    group.id > 0
                                  }
                                >
                                  <EndOfGroupDropZone
                                    groupId={group.id as number}
                                    instanceCount={group.instances.length}
                                  />
                                </Show>
                              </div>
                            </Collapsable>
                          </Show>
                        )
                      }}
                    </For>
                  </Show>
                </div>
              </ContextMenuTrigger>
              <ContextMenuContent>
                <ContextMenuGroup>
                  <ContextMenuGroupLabel>
                    <Trans key="library:_trn_add_new_instance" />
                  </ContextMenuGroupLabel>
                  <ContextMenuSeparator />
                  <ContextMenuItem
                    class="flex items-center gap-2"
                    onClick={() => {
                      modals?.openModal({
                        name: "instanceCreation"
                      })
                    }}
                  >
                    <div class="i-hugeicons:file-add h-4 w-4" />
                    <Trans key="library:_trn_create_new_instance" />
                  </ContextMenuItem>
                  <ContextMenuItem
                    class="flex items-center gap-2"
                    onClick={() => {
                      modals?.openModal(
                        {
                          name: "instanceCreation"
                        },
                        {
                          import: true
                        }
                      )
                    }}
                  >
                    <div class="i-hugeicons:download-02 h-4 w-4" />
                    <Trans key="library:_trn_import_instance" />
                  </ContextMenuItem>
                </ContextMenuGroup>
              </ContextMenuContent>
            </ContextMenu>
          </div>
        </Match>
      </Switch>
      <SelectionActionBar
        selectedCount={() => selectedIds().size}
        onClearSelection={clearSelection}
        onDelete={handleBatchDelete}
      />
      <Show when={dragSelect.selectionRect()}>
        {(rect) => (
          <div
            class="fixed pointer-events-none border-2 border-primary-500 bg-primary-500/20 z-50"
            style={{
              left: `${rect().left}px`,
              top: `${rect().top}px`,
              width: `${rect().width}px`,
              height: `${rect().height}px`
            }}
          />
        )}
      </Show>
      <DragGhost
        instances={globalStore.instances.data || []}
        groups={libraryItems()
          .filter(
            (
              item
            ): item is {
              id: string
              type: "folder"
              data: {
                id: number
                name: string
                libraryPosition: number | null
                instances: ListInstance[]
              }
            } => item.type === "folder"
          )
          .map((item) => ({
            id: item.data.id,
            name: item.data.name,
            instances: item.data.instances
          }))}
      />
    </div>
  )
}

export default HomeGrid
