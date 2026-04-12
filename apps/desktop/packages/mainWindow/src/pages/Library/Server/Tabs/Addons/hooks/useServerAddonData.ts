import { createSignal, createMemo, createEffect, onMount } from "solid-js"
import { createStore, reconcile } from "solid-js/store"
import { useParams } from "@solidjs/router"
import { rspc } from "@/utils/rspcClient"
import { ServerAddon } from "@gd/core_module/bindings"
import { useAddonTableState } from "@/pages/Library/shared/addons/hooks"

const SERVER_ADDON_TYPES = ["mods", "datapacks"] as const

interface FilterCacheEntry {
  enabledAddonTypes: Record<string, boolean>
  searchQuery: string
}

const defaultEnabledTypes = (): Record<string, boolean> => {
  const types: Record<string, boolean> = {}
  for (const t of SERVER_ADDON_TYPES) {
    types[t] = true
  }
  return types
}

// Module-level cache for filter state across component mounts
const filterCache = new Map<string, FilterCacheEntry>()

export const useServerAddonData = () => {
  const params = useParams()
  const paramId = params.id ?? ""
  const cached = filterCache.get(paramId)

  const [searchQuery, _setSearchQuery] = createSignal(cached?.searchQuery ?? "")
  const [enabledAddonTypes, setEnabledAddonTypesStore] = createStore<
    Record<string, boolean>
  >(cached?.enabledAddonTypes ?? defaultEnabledTypes())

  // Helper to get or init cache entry
  const getCache = (): FilterCacheEntry => {
    let entry = filterCache.get(paramId)
    if (!entry) {
      entry = {
        enabledAddonTypes: defaultEnabledTypes(),
        searchQuery: ""
      }
      filterCache.set(paramId, entry)
    }
    return entry
  }

  // Wrapped setters that imperatively sync to cache
  const setSearchQuery = (query: string) => {
    _setSearchQuery(query)
    getCache().searchQuery = query
  }

  const setEnabledAddonTypes = (type: string, enabled: boolean) => {
    setEnabledAddonTypesStore(type, enabled)
    getCache().enabledAddonTypes[type] = enabled
  }

  // Table states
  const tableState = useAddonTableState()

  // Reconciled store for addons to maintain stable object references
  const [addonsStore, setAddonsStore] = createStore<ServerAddon[]>([])

  const allAddons = rspc.createQuery(() => ({
    queryKey: ["server.getServerAddons", parseInt(paramId, 10)]
  }))

  // Prioritize caching for this server when the addons tab is mounted
  const prioritizeCache = rspc.createMutation(() => ({
    mutationKey: ["server.prioritizeServerCache"]
  }))

  onMount(() => {
    const serverId = parseInt(paramId, 10)
    if (!isNaN(serverId)) {
      prioritizeCache.mutate(serverId)
    }
  })

  // Reconcile addons data to maintain stable object references
  createEffect(() => {
    if (allAddons.data) {
      setAddonsStore(reconcile(allAddons.data, { key: "id" }))
    }
  })

  // Optimistic update functions
  const optimisticToggleAddon = (addonId: string, enabled: boolean) => {
    setAddonsStore((addon) => addon.id === addonId, "enabled", enabled)
  }

  const optimisticDeleteAddon = (addonId: string) => {
    const filtered = addonsStore.filter((addon) => addon.id !== addonId)
    setAddonsStore(reconcile(filtered, { key: "id" }))
  }

  const optimisticDeleteAddons = (addonIds: string[]) => {
    const filtered = addonsStore.filter((addon) => !addonIds.includes(addon.id))
    setAddonsStore(reconcile(filtered, { key: "id" }))
  }

  const rollbackToServerState = () => {
    if (allAddons.data) {
      setAddonsStore(reconcile(allAddons.data, { key: "id" }))
    }
  }

  // Filtered data based on type and search query
  const filteredAddons = createMemo(() => {
    return addonsStore.filter((addon) => {
      // Filter by addon type (multi-select)
      if (!enabledAddonTypes[addon.addonType]) return false

      // Filter by search query
      const query = searchQuery().toLowerCase()
      if (query) {
        const name = addon.displayName || addon.filename
        return name.toLowerCase().includes(query)
      }

      return true
    })
  })

  return {
    // Data
    allAddons,
    addonsStore,
    filteredAddons,

    // Filter states
    searchQuery,
    setSearchQuery,
    enabledAddonTypes,
    setEnabledAddonTypes,

    // Table states (from shared hook)
    ...tableState,

    // Optimistic updates
    optimisticToggleAddon,
    optimisticDeleteAddon,
    optimisticDeleteAddons,
    rollbackToServerState
  }
}
