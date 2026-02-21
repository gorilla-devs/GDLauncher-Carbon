/**
 * useLibraryData Hook
 *
 * Manages library data with stable object identity using reconcile.
 * Computes libraryItems (folders mode) and virtualGroups (accordion mode).
 */

import { createMemo, createEffect, on, Accessor } from "solid-js"
import { createStore, reconcile } from "solid-js/store"
import { useTransContext } from "@gd/i18n"
import { rspc } from "@/utils/rspcClient"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { ListInstance, ValidListInstance } from "@gd/core_module/bindings"
import {
  LibraryItem,
  VirtualGroup,
  LibraryViewMode,
  getViewMode
} from "../types"

interface LibraryDataStore {
  libraryItems: LibraryItem[]
  virtualGroups: VirtualGroup[]
  favoriteIds: number[]
}

export interface UseLibraryDataReturn {
  /** Reconciled library items for folders view */
  libraryItems: LibraryItem[]
  /** Reconciled virtual groups for accordion view */
  virtualGroups: VirtualGroup[]
  /** Favorite instance IDs */
  favoriteIds: number[]
  /** Current view mode */
  viewMode: Accessor<LibraryViewMode>
  /** Whether folders view is active */
  isFoldersView: Accessor<boolean>
  /** Default group ID for ungrouped instances */
  defaultGroupId: Accessor<number | null>
  /** Loading state */
  isLoading: Accessor<boolean>
  /** Whether library is empty */
  isEmpty: Accessor<boolean>
}

export function useLibraryData(filter: Accessor<string>): UseLibraryDataReturn {
  const [t] = useTransContext()
  const globalStore = useGlobalStore()

  // Query for default group ID
  const defaultGroupQuery = rspc.createQuery(() => ({
    queryKey: ["instance.getDefaultGroup"]
  }))

  const defaultGroupId = createMemo(() => defaultGroupQuery.data ?? null)

  // View mode derived from settings
  const viewMode = createMemo<LibraryViewMode>(() =>
    getViewMode(globalStore.settings.data?.instancesGroupBy)
  )

  const isFoldersView = createMemo(() => viewMode() === "folders")

  // Store with reconcile for stable object identity
  const [store, setStore] = createStore<LibraryDataStore>({
    libraryItems: [],
    virtualGroups: [],
    favoriteIds: []
  })

  // Compute favorite IDs
  createEffect(() => {
    const nameFilter = filter().replaceAll(" ", "").toLowerCase()
    const ids = (globalStore.instances.data || [])
      .filter(
        (i) =>
          i.favorite &&
          i.name.toLowerCase().replaceAll(" ", "").includes(nameFilter)
      )
      .map((i) => i.id)
    setStore("favoriteIds", reconcile(ids))
  })

  // Compute library items for folders mode
  createEffect(
    on(
      () => [
        globalStore.instances.data,
        globalStore.instanceGroups.data,
        filter(),
        defaultGroupId(),
        isFoldersView()
      ] as const,
      ([instances, groups, filterValue, defGroupId, foldersView]) => {
        const filterActive = !!filterValue?.trim()
        if (!foldersView || filterActive) {
          setStore("libraryItems", reconcile([], { key: "id" }))
          return
        }

        // Don't clear libraryItems while defaultGroupId is still loading —
        // keep previous data visible. isLoading covers the loading state.
        if (!defGroupId) return

        const items = computeLibraryItems(
          instances || [],
          groups || [],
          filterValue,
          defGroupId,
          t
        )
        setStore("libraryItems", reconcile(items, { key: "id" }))
      }
    )
  )

  // Compute virtual groups for accordion mode
  createEffect(
    on(
      () => [
        globalStore.instances.data,
        globalStore.instanceGroups.data,
        globalStore.settings.data,
        filter(),
        isFoldersView()
      ] as const,
      ([instances, groups, settings, filterValue, foldersView]) => {
        const filterActive = !!filterValue?.trim()
        if (foldersView && !filterActive) {
          setStore("virtualGroups", reconcile([], { key: "id" }))
          return
        }

        const virtualGroups = computeVirtualGroups(
          instances || [],
          groups || [],
          settings,
          filterValue,
          t
        )
        setStore("virtualGroups", reconcile(virtualGroups, { key: "id" }))
      }
    )
  )

  const isLoading = createMemo(
    () =>
      globalStore.instances.isLoading ||
      (isFoldersView() && !defaultGroupId())
  )

  const isEmpty = createMemo(
    () =>
      (globalStore.instances.data?.length || 0) === 0 &&
      !globalStore.instances.isLoading
  )

  // Return getters so store properties are read lazily in reactive contexts.
  // Reading store.libraryItems eagerly (e.g. `libraryItems: store.libraryItems`)
  // captures the proxy reference once — if reconcile replaces it, consumers
  // hold a stale proxy and never see updates.
  return {
    get libraryItems() { return store.libraryItems },
    get virtualGroups() { return store.virtualGroups },
    get favoriteIds() { return store.favoriteIds },
    viewMode,
    isFoldersView,
    defaultGroupId,
    isLoading,
    isEmpty
  }
}

/**
 * Compute library items for folders mode.
 * Returns ungrouped instances + folder tiles sorted by libraryPosition.
 */
function computeLibraryItems(
  instances: ListInstance[],
  groups: { id: number; name: string; library_position: number | null }[],
  filterValue: string,
  defaultGroupId: number,
  t: ReturnType<typeof import("@gd/i18n").useTransContext>[0]
): LibraryItem[] {
  const items: LibraryItem[] = []
  const nameFilter = filterValue.replaceAll(" ", "").toLowerCase()

  // Group instances by group_id
  const instancesByGroup = new Map<number, ListInstance[]>()
  for (const instance of instances) {
    const list = instancesByGroup.get(instance.group_id) || []
    list.push(instance)
    instancesByGroup.set(instance.group_id, list)
  }

  for (const group of groups) {
    const groupInstances = instancesByGroup.get(group.id) || []
    // Filter instances by name
    const filteredInstances = groupInstances.filter((inst) =>
      inst.name.toLowerCase().replaceAll(" ", "").includes(nameFilter)
    )

    if (group.id === defaultGroupId) {
      // Default group instances become ungrouped items (excluding favorites)
      for (const instance of filteredInstances) {
        if (!instance.favorite) {
          items.push({
            id: `instance-${instance.id}`,
            type: "instance",
            data: instance
          })
        }
      }
    } else {
      // Other groups become folder items
      const nonFavoriteInstances = groupInstances.filter((inst) => !inst.favorite)
      const hasAnyInstances = nonFavoriteInstances.length > 0
      const filteredNonFavorites = filteredInstances.filter((inst) => !inst.favorite)
      const hasFilteredInstances = filteredNonFavorites.length > 0

      // Show folder if it has no instances (empty folder) or has filtered instances
      if (!hasAnyInstances || hasFilteredInstances) {
        const folderName =
          group.name === "localize➽default"
            ? t("general:_trn_default")
            : group.name

        items.push({
          id: `folder-${group.id}`,
          type: "folder",
          data: {
            id: group.id,
            name: folderName,
            libraryPosition: group.library_position,
            instances: filteredNonFavorites
          }
        })
      }
    }
  }

  // Sort items by libraryPosition
  items.sort((a, b) => {
    const getKey = (item: LibraryItem) => {
      if (item.type === "instance") {
        return item.data.library_position ?? item.data.index
      }
      return item.data.libraryPosition ?? 10000
    }
    return getKey(a) - getKey(b)
  })

  return items
}

/**
 * Compute virtual groups for accordion mode.
 * Groups instances by the selected groupBy criteria.
 */
function computeVirtualGroups(
  instances: ListInstance[],
  _groups: { id: number; name: string }[],
  settings: { instancesGroupBy?: string | null; instancesSortBy?: string | null; instancesSortByAsc?: boolean; instancesGroupByAsc?: boolean } | undefined,
  filterValue: string,
  t: ReturnType<typeof import("@gd/i18n").useTransContext>[0]
): VirtualGroup[] {
  const nameFilter = filterValue.replaceAll(" ", "").toLowerCase()
  const groupBy = settings?.instancesGroupBy
  const sortBy = settings?.instancesSortBy
  const sortByAsc = settings?.instancesSortByAsc ?? true
  const groupByAsc = settings?.instancesGroupByAsc ?? true

  const groupsMap: Map<string, VirtualGroup> = new Map()

  if (groupBy === null || groupBy === undefined) {
    // Flat search results — no grouping, single group
    const matching = instances.filter(
      (inst) => inst.name.toLowerCase().replaceAll(" ", "").includes(nameFilter)
    )
    matching.sort((a, b) => a.name.localeCompare(b.name))
    return matching.length > 0
      ? [{ id: "search-results", name: t("library:_trn_search_results"), instances: matching }]
      : []
  }

  for (const instance of instances) {
    const validInstance =
      instance.status.status === "valid" ? instance.status.value : undefined

    let groupName: string | null = null
    let groupId: string | number | null = null

    if (groupBy === "gameVersion") {
      groupName = validInstance?.mc_version || null
      groupId = groupName
    } else if (groupBy === "modloader") {
      groupName = validInstance?.modloader || "vanilla"
      groupId = groupName
    } else if (groupBy === "modplatform") {
      groupName = validInstance?.modpack?.type || "No Platform"
      groupId = groupName
    }

    if (!groupName) continue

    // Filter by name
    if (!instance.name.toLowerCase().replaceAll(" ", "").includes(nameFilter)) {
      continue
    }

    if (!groupsMap.has(groupName)) {
      groupsMap.set(groupName, {
        id: groupId,
        name: groupName,
        instances: []
      })
    }

    groupsMap.get(groupName)!.instances.push(instance)
  }

  // Sort instances within each group
  for (const group of groupsMap.values()) {
    group.instances.sort((a, b) => {
      let result = sortInstances(a, b, sortBy)
      if (!sortByAsc) result = -result
      return result || a.name.localeCompare(b.name)
    })
  }

  // Convert to array and sort groups
  const result = Array.from(groupsMap.values())

  if (groupBy === "gameVersion") {
    result.sort((a, b) => {
      const cmp = a.name.localeCompare(b.name, undefined, {
        numeric: true,
        sensitivity: "base"
      })
      return groupByAsc ? cmp : -cmp
    })
  } else {
    result.sort((a, b) => {
      const cmp = a.name.localeCompare(b.name)
      return groupByAsc ? cmp : -cmp
    })
  }

  return result
}

/**
 * Sort instances by the given criteria.
 */
function sortInstances(
  a: ListInstance,
  b: ListInstance,
  sortBy: string | null | undefined
): number {
  if (sortBy === null || sortBy === undefined) {
    return a.index - b.index
  }

  const validA = a.status.status === "valid" ? (a.status.value as ValidListInstance) : undefined
  const validB = b.status.status === "valid" ? (b.status.value as ValidListInstance) : undefined

  switch (sortBy) {
    case "name":
      return a.name.localeCompare(b.name)
    case "mostPlayed":
      return (a.seconds_played || 0) - (b.seconds_played || 0)
    case "lastPlayed": {
      const aTime = a.last_played ? Date.parse(a.last_played) : 0
      const bTime = b.last_played ? Date.parse(b.last_played) : 0
      return aTime - bTime
    }
    case "lastUpdated": {
      const aTime = a.date_updated ? Date.parse(a.date_updated) : 0
      const bTime = b.date_updated ? Date.parse(b.date_updated) : 0
      return aTime - bTime
    }
    case "gameVersion":
      return (validA?.mc_version || "").localeCompare(
        validB?.mc_version || "",
        undefined,
        { numeric: true, sensitivity: "base" }
      )
    case "created": {
      const aTime = a.date_created ? Date.parse(a.date_created) : 0
      const bTime = b.date_created ? Date.parse(b.date_created) : 0
      return aTime - bTime
    }
    default:
      return 0
  }
}
