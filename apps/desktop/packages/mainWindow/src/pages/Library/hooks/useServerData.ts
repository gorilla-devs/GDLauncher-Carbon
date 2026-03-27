/**
 * useServerData Hook
 *
 * Mirrors useLibraryData but for servers. Computes libraryItems with server tiles.
 */

import { createMemo, createEffect, on, Accessor } from "solid-js"
import { createStore, reconcile } from "solid-js/store"
import { rspc } from "@/utils/rspcClient"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { ListServer } from "@gd/core_module/bindings"
import {
  LibraryItem,
  VirtualGroup,
  LibraryViewMode,
  getViewMode
} from "../types"

interface ServerDataStore {
  libraryItems: LibraryItem[]
  virtualGroups: VirtualGroup[]
  favoriteIds: number[]
}

export interface UseServerDataReturn {
  libraryItems: LibraryItem[]
  virtualGroups: VirtualGroup[]
  favoriteIds: number[]
  viewMode: Accessor<LibraryViewMode>
  isFoldersView: Accessor<boolean>
  defaultGroupId: Accessor<number | null>
  isLoading: Accessor<boolean>
  isEmpty: Accessor<boolean>
}

export function useServerData(filter: Accessor<string>): UseServerDataReturn {
  const globalStore = useGlobalStore()

  const defaultGroupQuery = rspc.createQuery(() => ({
    queryKey: ["server.getDefaultGroup"]
  }))

  const defaultGroupId = createMemo(() => defaultGroupQuery.data ?? null)

  // Server view mode from settings
  const viewMode = createMemo<LibraryViewMode>(() =>
    getViewMode(globalStore.settings.data?.serversGroupBy)
  )

  const isFoldersView = createMemo(() => viewMode() === "folders")

  const [store, setStore] = createStore<ServerDataStore>({
    libraryItems: [],
    virtualGroups: [],
    favoriteIds: []
  })

  // Compute favorite IDs
  createEffect(() => {
    const nameFilter = filter().replaceAll(" ", "").toLowerCase()
    const ids = (globalStore.servers.data || [])
      .filter(
        (s) =>
          s.favorite &&
          s.name.toLowerCase().replaceAll(" ", "").includes(nameFilter)
      )
      .map((s) => s.id)
    setStore("favoriteIds", reconcile(ids))
  })

  // Compute library items for folders mode
  createEffect(
    on(
      () => [
        globalStore.servers.data,
        globalStore.serverGroups.data,
        filter(),
        defaultGroupId(),
        isFoldersView()
      ] as const,
      ([servers, groups, filterValue, defGroupId, foldersView]) => {
        const filterActive = !!filterValue?.trim()
        if (!foldersView || filterActive) {
          setStore("libraryItems", reconcile([], { key: "id" }))
          return
        }

        if (!defGroupId) return

        const items = computeServerLibraryItems(
          servers || [],
          groups || [],
          filterValue,
          defGroupId
        )
        setStore("libraryItems", reconcile(items, { key: "id" }))
      }
    )
  )

  // Compute virtual groups for accordion/search mode
  createEffect(
    on(
      () => [
        globalStore.servers.data,
        globalStore.serverGroups.data,
        globalStore.settings.data,
        filter(),
        isFoldersView()
      ] as const,
      ([servers, _groups, settings, filterValue, foldersView]) => {
        const filterActive = !!filterValue?.trim()
        if (foldersView && !filterActive) {
          setStore("virtualGroups", reconcile([], { key: "id" }))
          return
        }

        const virtualGroups = computeServerVirtualGroups(
          servers || [],
          settings,
          filterValue
        )
        setStore("virtualGroups", reconcile(virtualGroups, { key: "id" }))
      }
    )
  )

  const isLoading = createMemo(
    () =>
      globalStore.servers.isLoading ||
      (isFoldersView() && !defaultGroupId())
  )

  const isEmpty = createMemo(
    () =>
      (globalStore.servers.data?.length || 0) === 0 &&
      !globalStore.servers.isLoading
  )

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

function computeServerLibraryItems(
  servers: ListServer[],
  groups: { id: number; name: string; libraryPosition: number | null }[],
  filterValue: string,
  defaultGroupId: number
): LibraryItem[] {
  const items: LibraryItem[] = []
  const nameFilter = filterValue.replaceAll(" ", "").toLowerCase()

  // Group servers by group_id
  const serversByGroup = new Map<number, ListServer[]>()
  for (const server of servers) {
    const list = serversByGroup.get(server.groupId) || []
    list.push(server)
    serversByGroup.set(server.groupId, list)
  }

  for (const group of groups) {
    const groupServers = serversByGroup.get(group.id) || []
    const filteredServers = groupServers.filter((s) =>
      s.name.toLowerCase().replaceAll(" ", "").includes(nameFilter)
    )

    if (group.id === defaultGroupId) {
      for (const server of filteredServers) {
        items.push({
          id: `server-${server.id}`,
          type: "server",
          data: server
        })
      }
    } else if (groupServers.length === 0 || filteredServers.length > 0) {
      // Show as folder — servers inside are rendered by ExpandedFolderContent
      items.push({
        id: `folder-${group.id}`,
        type: "folder",
        data: {
          id: group.id,
          name: group.name,
          libraryPosition: group.libraryPosition,
          instances: filteredServers
        }
      })
    }
  }

  // Sort by libraryPosition
  items.sort((a, b) => {
    const getKey = (item: LibraryItem) => {
      if (item.type === "server") {
        return item.data.libraryPosition ?? item.data.index
      }
      if (item.type === "folder") {
        return item.data.libraryPosition ?? 10000
      }
      return 10000
    }
    return getKey(a) - getKey(b)
  })

  return items
}

function computeServerVirtualGroups(
  servers: ListServer[],
  settings: { serversGroupBy?: string | null; serversSortBy?: string | null; serversSortByAsc?: boolean; serversGroupByAsc?: boolean } | undefined,
  filterValue: string
): VirtualGroup[] {
  const nameFilter = filterValue.replaceAll(" ", "").toLowerCase()
  const groupBy = settings?.serversGroupBy
  const sortByAsc = settings?.serversSortByAsc ?? true
  const groupByAsc = settings?.serversGroupByAsc ?? true

  const matching = servers.filter(
    (s) => s.name.toLowerCase().replaceAll(" ", "").includes(nameFilter)
  )

  if (groupBy === null || groupBy === undefined) {
    // Flat search results or folders mode — single group
    matching.sort((a, b) => a.name.localeCompare(b.name))
    return matching.length > 0
      ? [{ id: "search-results", name: "Search Results", instances: [] }]
      : []
  }

  // Group by gameVersion is the only meaningful grouping for servers initially
  const groupsMap = new Map<string, VirtualGroup>()

  for (const server of matching) {
    let groupName = server.gameVersion || "Unknown"
    let groupId: string = groupName

    if (groupBy === "gameVersion") {
      groupName = server.gameVersion || "Unknown"
      groupId = groupName
    }

    if (!groupsMap.has(groupName)) {
      groupsMap.set(groupName, {
        id: groupId,
        name: groupName,
        instances: [] // VirtualGroup uses instances field
      })
    }
  }

  const result = Array.from(groupsMap.values())
  result.sort((a, b) => {
    const cmp = a.name.localeCompare(b.name, undefined, { numeric: true })
    return groupByAsc ? cmp : -cmp
  })

  return result
}
