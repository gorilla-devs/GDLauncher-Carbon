/**
 * Pure computation helpers for the servers library views.
 *
 * Mirrors the instance-side logic in useLibraryData: computeServerLibraryItems
 * feeds the folders view, computeServerVirtualGroups feeds the accordion view
 * and flat search results.
 */

import { ListServer } from "@gd/core_module/bindings"
import { LibraryItem, VirtualGroup } from "../types"

export function computeServerLibraryItems(
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

  // Folders always come before ungrouped servers, regardless of
  // libraryPosition. Within each bucket, sort ascending by libraryPosition
  // (falling back to index for servers).
  items.sort((a, b) => {
    const aFolder = a.type === "folder"
    const bFolder = b.type === "folder"
    if (aFolder !== bFolder) return aFolder ? -1 : 1
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

export function computeServerVirtualGroups(
  servers: ListServer[],
  settings:
    | {
        instancesGroupBy?: string | null
        instancesSortBy?: string | null
        instancesSortByAsc?: boolean
        instancesGroupByAsc?: boolean
      }
    | undefined,
  filterValue: string,
  searchResultsLabel: string
): VirtualGroup<ListServer>[] {
  const nameFilter = filterValue.replaceAll(" ", "").toLowerCase()
  const groupBy = settings?.instancesGroupBy
  const sortBy = settings?.instancesSortBy
  const sortByAsc = settings?.instancesSortByAsc ?? true
  const groupByAsc = settings?.instancesGroupByAsc ?? true

  const matching = servers.filter((s) =>
    s.name.toLowerCase().replaceAll(" ", "").includes(nameFilter)
  )

  if (groupBy === null || groupBy === undefined) {
    // Flat search results in folders mode — single group
    matching.sort((a, b) => a.name.localeCompare(b.name))
    return matching.length > 0
      ? [
          {
            id: "search-results",
            name: searchResultsLabel,
            instances: matching
          }
        ]
      : []
  }

  // Servers only support grouping by game version, but the setting is shared
  // with the instances library and can hold instance-only values (modloader,
  // modplatform) — those fall back to game version grouping too.
  const groupsMap = new Map<string, VirtualGroup<ListServer>>()

  for (const server of matching) {
    const groupName = server.gameVersion || "Unknown"

    if (!groupsMap.has(groupName)) {
      groupsMap.set(groupName, {
        id: groupName,
        name: groupName,
        instances: []
      })
    }

    groupsMap.get(groupName)!.instances.push(server)
  }

  // Sort servers within each group
  for (const group of groupsMap.values()) {
    group.instances.sort((a, b) => {
      let result = sortServers(a, b, sortBy)
      if (!sortByAsc) result = -result
      return result || a.name.localeCompare(b.name)
    })
  }

  const result = Array.from(groupsMap.values())
  result.sort((a, b) => {
    const cmp = a.name.localeCompare(b.name, undefined, {
      numeric: true,
      sensitivity: "base"
    })
    return groupByAsc ? cmp : -cmp
  })

  return result
}

/**
 * Sort servers by the shared instancesSortBy setting. Server mode only offers
 * name, gameVersion and created; instance-only criteria fall through to the
 * caller's name fallback.
 */
function sortServers(
  a: ListServer,
  b: ListServer,
  sortBy: string | null | undefined
): number {
  if (sortBy === null || sortBy === undefined) {
    return a.index - b.index
  }

  switch (sortBy) {
    case "name":
      return a.name.localeCompare(b.name)
    case "gameVersion":
      return a.gameVersion.localeCompare(b.gameVersion, undefined, {
        numeric: true,
        sensitivity: "base"
      })
    case "created": {
      const aTime = a.dateCreated ? Date.parse(a.dateCreated) : 0
      const bTime = b.dateCreated ? Date.parse(b.dateCreated) : 0
      return aTime - bTime
    }
    default:
      return 0
  }
}
