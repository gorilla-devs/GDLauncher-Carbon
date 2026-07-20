/**
 * useServerData Hook
 *
 * Mirrors useLibraryData but for servers. Computes libraryItems with server tiles.
 */

import { createMemo, createEffect, on, Accessor } from "solid-js"
import { createStore, reconcile } from "solid-js/store"
import { useTransContext } from "@gd/i18n"
import { rspc } from "@/utils/rspcClient"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { ListServer } from "@gd/core_module/bindings"
import {
  LibraryItem,
  VirtualGroup,
  LibraryViewMode,
  getViewMode
} from "../types"
import {
  computeServerLibraryItems,
  computeServerVirtualGroups
} from "../utils/serverGrouping"

interface ServerDataStore {
  libraryItems: LibraryItem[]
  virtualGroups: VirtualGroup<ListServer>[]
  favoriteIds: number[]
}

export interface UseServerDataReturn {
  libraryItems: LibraryItem[]
  virtualGroups: VirtualGroup<ListServer>[]
  favoriteIds: number[]
  viewMode: Accessor<LibraryViewMode>
  isFoldersView: Accessor<boolean>
  defaultGroupId: Accessor<number | null>
  isLoading: Accessor<boolean>
  isEmpty: Accessor<boolean>
}

export function useServerData(filter: Accessor<string>): UseServerDataReturn {
  const [t] = useTransContext()
  const globalStore = useGlobalStore()

  const defaultGroupQuery = rspc.createQuery(() => ({
    queryKey: ["server.getDefaultGroup"]
  }))

  const defaultGroupId = createMemo(() => defaultGroupQuery.data ?? null)

  // Server view mode from settings (servers share the instances* settings)
  const viewMode = createMemo<LibraryViewMode>(() =>
    getViewMode(globalStore.settings.data?.instancesGroupBy)
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
      () =>
        [
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
      () =>
        [
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
          filterValue,
          t("library:_trn_search_results")
        )
        setStore("virtualGroups", reconcile(virtualGroups, { key: "id" }))
      }
    )
  )

  const isLoading = createMemo(
    () =>
      globalStore.servers.isLoading || (isFoldersView() && !defaultGroupId())
  )

  const isEmpty = createMemo(
    () =>
      (globalStore.servers.data?.length || 0) === 0 &&
      !globalStore.servers.isLoading
  )

  return {
    get libraryItems() {
      return store.libraryItems
    },
    get virtualGroups() {
      return store.virtualGroups
    },
    get favoriteIds() {
      return store.favoriteIds
    },
    viewMode,
    isFoldersView,
    defaultGroupId,
    isLoading,
    isEmpty
  }
}
