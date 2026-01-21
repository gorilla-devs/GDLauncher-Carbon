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
import GroupHeader from "@/components/Library/GroupHeader"
import FolderTile, {
  clickedFolderId,
  setClickedFolderId
} from "@/components/Library/FolderTile"
import ExpandedFolderContent from "@/components/Library/ExpandedFolderContent"
import "@/components/Library/folderTransitions.css"

const animatedInstanceIds = new Set<number>()
let initialAnimationComplete = false

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

  // Store refs for all instance tiles
  const tileRefs = new Map<number, HTMLDivElement>()

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
    // Also ignore if drag and drop is active
    return (
      !dragContext.dragSelectEnabled() ||
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

  const toggleFolder = (folderId: number) => {
    const shouldTransition =
      !globalStore.settings.data?.reducedMotion && document.startViewTransition

    if (shouldTransition) {
      setClickedFolderId(folderId)
      const transition = document.startViewTransition(() => {
        setOpenFolderId((prev) => (prev === folderId ? null : folderId))
      })
      transition.finished.then(() => setClickedFolderId(null))
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
              before: target.groupId
            })
          }
          break
        }
        case "endOfGroups": {
          // Move group to end
          moveGroupMutation.mutate({
            group: groupId,
            before: null
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
    | { type: "instance"; data: ListInstance }
    | {
        type: "folder"
        data: { id: number; name: string; instances: ListInstance[] }
      }

  // Get the default group ID from query
  const defaultGroupQuery = rspc.createQuery(() => ({
    queryKey: ["instance.getDefaultGroup"]
  }))

  const defaultGroupId = createMemo(() => defaultGroupQuery.data ?? null)

  // Favorite instances (static row at top)
  const favoriteInstances = createMemo(() => {
    const nameFilter = filter().replaceAll(" ", "").toLowerCase()
    return (globalStore.instances.data || []).filter(
      (i) =>
        i.favorite &&
        i.name.toLowerCase().replaceAll(" ", "").includes(nameFilter)
    )
  })

  // Library items: ungrouped instances + folder tiles (when in folder view mode)
  const libraryItems = createMemo((): LibraryItem[] => {
    const items: LibraryItem[] = []
    const nameFilter = filter().replaceAll(" ", "").toLowerCase()
    const _defaultGroupId = defaultGroupId()

    if (!_defaultGroupId) return items

    // Get all groups and instances from the store
    const groups = globalStore.instanceGroups.data || []
    const allInstances = globalStore.instances.data || []

    // Group instances by group_id
    const instancesByGroup = new Map<number, ListInstance[]>()
    for (const instance of allInstances) {
      const list = instancesByGroup.get(instance.group_id) || []
      list.push(instance)
      instancesByGroup.set(instance.group_id, list)
    }

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
            items.push({ type: "instance", data: instance })
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
          items.push({
            type: "folder",
            data: {
              id: group.id,
              name:
                group.name === "localize➽default"
                  ? t("general:_trn_default")
                  : group.name,
              instances: filteredNonFavorites
            }
          })
        }
      }
    }

    // Sort items: instances by their index, folders by their group index
    // Get group indices for folders (use position in array as proxy)
    const groupIndexMap = new Map<number, number>()
    groups.forEach((g, index) => {
      groupIndexMap.set(g.id, index)
    })

    items.sort((a, b) => {
      // Get sort keys
      const getKey = (item: LibraryItem) => {
        if (item.type === "instance") {
          return item.data.index
        } else {
          return (groupIndexMap.get(item.data.id) ?? 0) * 10000 // Folders interleave with instances
        }
      }
      return getKey(a) - getKey(b)
    })

    return items
  })

  // Check if we're in iOS-style folder view mode
  const isIosFolderView = createMemo(
    () => globalStore.settings.data?.instancesGroupBy === "group"
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
        <Match when={globalStore.instances.isLoading}>
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
                <div class="mt-4" onClick={() => setOpenFolderId(null)}>
                  {/* iOS-style folder view when grouped by "group" */}
                  <Show when={isIosFolderView()}>
                    {/* Favorites Row (static, non-draggable) */}
                    <Show when={favoriteInstances().length > 0}>
                      <div class="mb-6">
                        <div class="flex items-center gap-2 mb-3">
                          <div class="i-ri:star-fill text-yellow-500" />
                          <span class="text-sm font-medium text-lightSlate-500 uppercase">
                            <Trans key="instances:_trn_favorites" />
                          </span>
                        </div>
                        <div
                          class="flex flex-wrap gap-x-4"
                          classList={{
                            "gap-y-4": instancesTileSize() === 1,
                            "gap-y-6": instancesTileSize() === 2,
                            "gap-y-8": instancesTileSize() === 3,
                            "gap-y-10": instancesTileSize() === 4,
                            "gap-y-12": instancesTileSize() === 5
                          }}
                        >
                          <For each={favoriteInstances()}>
                            {(instance) => (
                              <div data-instance-tile class="relative">
                                <InstanceTile
                                  instance={instance}
                                  identifier={`favorites-${instance.id}`}
                                  size={instancesTileSize() as any}
                                  preventClick={() => dragContext.justDropped()}
                                />
                              </div>
                            )}
                          </For>
                        </div>
                      </div>
                      <div class="border-b border-darkSlate-600 mb-6" />
                    </Show>

                    {/* Main Grid: Ungrouped instances + Folder tiles */}
                    <div
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
                        {(item, itemIndex) => {
                          return (
                            <>
                              <Switch>
                                <Match
                                  when={item.type === "folder" && item.data}
                                >
                                  {(_) => {
                                    const folder = () =>
                                      (
                                        item as {
                                          type: "folder"
                                          data: {
                                            id: number
                                            name: string
                                            instances: ListInstance[]
                                          }
                                        }
                                      ).data
                                    let folderRef: HTMLDivElement | undefined

                                    // Show drop indicator before this folder when dragging groups
                                    const showFolderDropIndicator = () => {
                                      const target = dragContext.dropTarget()
                                      return (
                                        dragContext.isDragging() &&
                                        dragContext.dragType() === "group" &&
                                        target?.type === "beforeGroup" &&
                                        target.groupId === folder().id
                                      )
                                    }

                                    // Register drop zone for folder reordering
                                    createEffect(() => {
                                      if (
                                        dragContext.isDragging() &&
                                        dragContext.dragType() === "group" &&
                                        folderRef
                                      ) {
                                        // Don't register drop zone for the folder being dragged
                                        if (
                                          dragContext
                                            .draggedIds()
                                            .includes(folder().id)
                                        ) {
                                          dragContext.unregisterDropZone(
                                            `before-group-${folder().id}`
                                          )
                                          return
                                        }

                                        const rect =
                                          folderRef.getBoundingClientRect()
                                        // Register drop zone on left edge
                                        const dropRect = new DOMRect(
                                          rect.left - 8,
                                          rect.top,
                                          rect.width / 3 + 8,
                                          rect.height
                                        )
                                        dragContext.registerDropZone({
                                          id: `before-group-${folder().id}`,
                                          rect: dropRect,
                                          target: {
                                            type: "beforeGroup",
                                            groupId: folder().id
                                          }
                                        })
                                      } else {
                                        dragContext.unregisterDropZone(
                                          `before-group-${folder().id}`
                                        )
                                      }
                                    })

                                    onCleanup(() => {
                                      dragContext.unregisterDropZone(
                                        `before-group-${folder().id}`
                                      )
                                    })

                                    return (
                                      <div
                                        ref={folderRef}
                                        class="relative"
                                        onClick={(e) => e.stopPropagation()}
                                      >
                                        {/* Drop indicator before this folder */}
                                        <Show when={showFolderDropIndicator()}>
                                          <div class="absolute -left-2 top-0 bottom-0 w-1 bg-primary-500 rounded-full z-50">
                                            <div class="absolute -top-1 left-1/2 -translate-x-1/2 w-2 h-2 rounded-full bg-primary-500" />
                                            <div class="absolute -bottom-1 left-1/2 -translate-x-1/2 w-2 h-2 rounded-full bg-primary-500" />
                                          </div>
                                        </Show>
                                        <FolderTile
                                          group={folder()}
                                          isOpen={
                                            openFolderId() === folder().id
                                          }
                                          onToggle={() =>
                                            toggleFolder(folder().id)
                                          }
                                          size={
                                            instancesTileSize() as
                                              | 1
                                              | 2
                                              | 3
                                              | 4
                                              | 5
                                          }
                                        />
                                      </div>
                                    )
                                  }}
                                </Match>
                                <Match
                                  when={item.type === "instance" && item.data}
                                >
                                  {(_) => {
                                    const instance = () =>
                                      (
                                        item as {
                                          type: "instance"
                                          data: ListInstance
                                        }
                                      ).data
                                    let ref: HTMLDivElement | undefined

                                    const isBeingDragged = () =>
                                      dragContext.isDragging() &&
                                      dragContext.dragType() === "instance" &&
                                      dragContext
                                        .draggedIds()
                                        .includes(instance().id)

                                    const showDropIndicator = () => {
                                      const target = dragContext.dropTarget()
                                      return (
                                        dragContext.isDragging() &&
                                        dragContext.dragType() === "instance" &&
                                        target?.type === "beforeInstance" &&
                                        target.instanceId === instance().id
                                      )
                                    }

                                    const showCreateFolderIndicator = () => {
                                      const target = dragContext.dropTarget()
                                      return (
                                        dragContext.isDragging() &&
                                        dragContext.dragType() === "instance" &&
                                        target?.type === "createFolder" &&
                                        target.instanceId === instance().id
                                      )
                                    }

                                    // Register drop zones for ungrouped instances
                                    createEffect(() => {
                                      if (
                                        dragContext.isDragging() &&
                                        dragContext.dragType() === "instance" &&
                                        ref
                                      ) {
                                        // Don't register drop zone for dragged instances
                                        if (
                                          dragContext
                                            .draggedIds()
                                            .includes(instance().id)
                                        ) {
                                          dragContext.unregisterDropZone(
                                            `before-instance-${instance().id}`
                                          )
                                          dragContext.unregisterDropZone(
                                            `create-folder-${instance().id}`
                                          )
                                          return
                                        }

                                        const rect = ref.getBoundingClientRect()

                                        // Register "before instance" drop zone (left edge)
                                        const dropRect = new DOMRect(
                                          rect.left - 8,
                                          rect.top,
                                          rect.width / 4 + 8,
                                          rect.height
                                        )
                                        dragContext.registerDropZone({
                                          id: `before-instance-${instance().id}`,
                                          rect: dropRect,
                                          target: {
                                            type: "beforeInstance",
                                            instanceId: instance().id,
                                            groupId: defaultGroupId()!
                                          }
                                        })

                                        // Register "create folder" drop zone (center of tile)
                                        const centerRect = new DOMRect(
                                          rect.left + rect.width * 0.25,
                                          rect.top,
                                          rect.width * 0.5,
                                          rect.height
                                        )
                                        dragContext.registerDropZone({
                                          id: `create-folder-${instance().id}`,
                                          rect: centerRect,
                                          target: {
                                            type: "createFolder",
                                            instanceId: instance().id
                                          }
                                        })
                                      } else {
                                        dragContext.unregisterDropZone(
                                          `before-instance-${instance().id}`
                                        )
                                        dragContext.unregisterDropZone(
                                          `create-folder-${instance().id}`
                                        )
                                      }
                                    })

                                    onCleanup(() => {
                                      dragContext.unregisterDropZone(
                                        `before-instance-${instance().id}`
                                      )
                                      dragContext.unregisterDropZone(
                                        `create-folder-${instance().id}`
                                      )
                                      tileRefs.delete(instance().id)
                                    })

                                    onMount(() => {
                                      if (ref) {
                                        tileRefs.set(instance().id, ref)
                                      }
                                    })

                                    return (
                                      <div
                                        ref={ref}
                                        data-instance-tile
                                        class="relative"
                                      >
                                        {/* Drop indicator before this instance */}
                                        <Show when={showDropIndicator()}>
                                          <div class="absolute -left-2 top-0 bottom-0 w-1 bg-primary-500 rounded-full z-50">
                                            <div class="absolute -top-1 left-1/2 -translate-x-1/2 w-2 h-2 rounded-full bg-primary-500" />
                                            <div class="absolute -bottom-1 left-1/2 -translate-x-1/2 w-2 h-2 rounded-full bg-primary-500" />
                                          </div>
                                        </Show>
                                        {/* Create folder indicator */}
                                        <Show
                                          when={showCreateFolderIndicator()}
                                        >
                                          <div class="absolute inset-0 border-2 border-primary-500 rounded-lg bg-primary-500/20 pointer-events-none z-40 flex items-center justify-center">
                                            <div class="i-hugeicons:folder-add text-primary-400 text-2xl" />
                                          </div>
                                        </Show>
                                        <InstanceTile
                                          instance={instance()}
                                          identifier={`ungrouped-${instance().id}`}
                                          size={instancesTileSize() as any}
                                          isMultiSelected={isSelected(
                                            instance().id
                                          )}
                                          onToggleSelection={() =>
                                            toggleSelection(instance().id)
                                          }
                                          isDragging={isBeingDragged()}
                                          groupId={
                                            defaultGroupId() ?? undefined
                                          }
                                          onDragStart={(e) => {
                                            const ids = isSelected(
                                              instance().id
                                            )
                                              ? Array.from(selectedIds())
                                              : [instance().id]
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
                                </Match>
                              </Switch>
                            </>
                          )
                        }}
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

                                    const baseDelay = 300

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
                                          <div class="absolute -left-2 top-0 bottom-0 w-1 bg-primary-500 rounded-full z-50">
                                            <div class="absolute -top-1 left-1/2 -translate-x-1/2 w-2 h-2 rounded-full bg-primary-500" />
                                            <div class="absolute -bottom-1 left-1/2 -translate-x-1/2 w-2 h-2 rounded-full bg-primary-500" />
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
        groups={
          globalStore.instanceGroups.data?.map((g) => ({
            id: g.id,
            name: g.name
          })) || []
        }
      />
    </div>
  )
}

export default HomeGrid
