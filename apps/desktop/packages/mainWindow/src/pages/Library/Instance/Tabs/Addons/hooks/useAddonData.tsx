import { createSignal, createMemo, createEffect, onMount } from "solid-js"
import { createStore, reconcile } from "solid-js/store"
import { useParams } from "@solidjs/router"
import { rspc } from "@/utils/rspcClient"
import { AddonType, Mod } from "@gd/core_module/bindings"
import { useAddonTableState } from "@/pages/Library/shared/addons/hooks"

type PlatformFilter = "all" | "curseforge" | "modrinth" | "local"

interface FilterCacheEntry {
  enabledAddonTypes: Record<AddonType, boolean>
  platformFilter: PlatformFilter
  searchQuery: string
}

const DEFAULT_ADDON_TYPES: Record<AddonType, boolean> = {
  mods: true,
  shaders: true,
  resourcepacks: true,
  datapacks: true,
  worlds: true
}

// Module-level cache for filter state across component mounts
const filterCache = new Map<string, FilterCacheEntry>()

export const useAddonData = () => {
  const params = useParams<{ id: string }>()
  const cached = filterCache.get(params.id)

  // Create fresh signals/store each mount
  const [searchQuery, _setSearchQuery] = createSignal(cached?.searchQuery ?? "")
  const [enabledAddonTypes, _setEnabledAddonTypes] = createStore<
    Record<AddonType, boolean>
  >({ ...DEFAULT_ADDON_TYPES })
  const [platformFilter, _setPlatformFilter] = createSignal<PlatformFilter>(
    cached?.platformFilter ?? "all"
  )

  // Restore addon types from cache after store creation
  if (cached) {
    for (const type of Object.keys(cached.enabledAddonTypes) as AddonType[]) {
      _setEnabledAddonTypes(type, cached.enabledAddonTypes[type])
    }
  }

  // Helper to get or init cache entry
  const getCache = (): FilterCacheEntry => {
    let entry = filterCache.get(params.id)
    if (!entry) {
      entry = {
        enabledAddonTypes: { ...DEFAULT_ADDON_TYPES },
        platformFilter: "all",
        searchQuery: ""
      }
      filterCache.set(params.id, entry)
    }
    return entry
  }

  // Wrapped setters that imperatively sync to cache
  const setSearchQuery = (query: string) => {
    _setSearchQuery(query)
    getCache().searchQuery = query
  }

  const setEnabledAddonTypes = (type: AddonType, enabled: boolean) => {
    _setEnabledAddonTypes(type, enabled)
    getCache().enabledAddonTypes[type] = enabled
  }

  const setPlatformFilter = (filter: PlatformFilter) => {
    _setPlatformFilter(filter)
    getCache().platformFilter = filter
  }

  // Table states
  const tableState = useAddonTableState()

  // Reconciled store for addons to maintain stable object references
  const [addonsStore, setAddonsStore] = createStore<Mod[]>([])

  // Track mods that are currently being updated
  const [updatingModIds, setUpdatingModIds] = createSignal<Set<string>>(
    new Set()
  )

  const [cachePrioritized, setCachePrioritized] = createSignal(false)

  const allAddons = rspc.createQuery(() => ({
    queryKey: ["instance.getInstanceMods", parseInt(params.id, 10)]
  }))

  const prioritizeCache = rspc.createMutation(() => ({
    mutationKey: ["instance.prioritizeInstanceCache"]
  }))

  onMount(() => {
    const instanceId = parseInt(params.id, 10)
    if (!isNaN(instanceId) && !cachePrioritized()) {
      setCachePrioritized(true)
      prioritizeCache.mutate(instanceId)
    }
  })

  // Reconcile addons data to maintain stable object references
  createEffect(() => {
    if (allAddons.data) {
      // Get current updating mods to preserve their state
      const currentlyUpdating = updatingModIds()

      // Map the data to preserve optimistic updates for mods being updated
      const dataWithOptimisticUpdates = allAddons.data.map((addon) => {
        if (currentlyUpdating.has(addon.id)) {
          // For mods being updated, keep showing them as not having updates
          // This prevents the button from reappearing during the update
          return { ...addon, has_update: false }
        }
        return addon
      })

      setAddonsStore(reconcile(dataWithOptimisticUpdates, { key: "id" }))

      // Clean up updatingModIds for mods that truly don't have updates anymore
      // (i.e., the update completed successfully)
      allAddons.data.forEach((addon) => {
        if (!addon.has_update && currentlyUpdating.has(addon.id)) {
          setUpdatingModIds((prev) => {
            const next = new Set(prev)
            next.delete(addon.id)
            return next
          })
        }
      })
    }
  })

  // Optimistic update functions
  const optimisticToggleAddon = (addonId: string, enabled: boolean) => {
    setAddonsStore((addon) => addon.id === addonId, "enabled", enabled)
  }

  const optimisticDeleteAddon = (addonId: string) => {
    const filteredAddons = addonsStore.filter((addon) => addon.id !== addonId)
    setAddonsStore(reconcile(filteredAddons, { key: "id" }))
  }

  const optimisticDeleteAddons = (addonIds: string[]) => {
    const filteredAddons = addonsStore.filter(
      (addon) => !addonIds.includes(addon.id)
    )
    setAddonsStore(reconcile(filteredAddons, { key: "id" }))
  }

  const optimisticUpdateAddon = (addonId: string) => {
    setAddonsStore((addon) => addon.id === addonId, "has_update", false)
  }

  const startUpdatingMod = (modId: string) => {
    setUpdatingModIds((prev) => {
      const next = new Set(prev)
      next.add(modId)
      return next
    })
  }

  const stopUpdatingMod = (modId: string) => {
    setUpdatingModIds((prev) => {
      const next = new Set(prev)
      next.delete(modId)
      return next
    })
  }

  const isModUpdating = (modId: string) => {
    return updatingModIds().has(modId)
  }

  const rollbackToServerState = () => {
    if (allAddons.data) {
      setAddonsStore(reconcile(allAddons.data, { key: "id" }))
      setUpdatingModIds(new Set<string>())
    }
  }

  // Filtered data based on enabled addon types and platform filter
  const filteredAddons = createMemo(() => {
    return addonsStore.filter((addon) => {
      // Filter by addon type
      const typeEnabled = enabledAddonTypes[addon.addon_type]
      if (!typeEnabled) return false

      // Filter by platform
      if (platformFilter() === "curseforge" && !addon.curseforge) return false
      if (platformFilter() === "modrinth" && !addon.modrinth) return false
      if (platformFilter() === "local" && (addon.curseforge || addon.modrinth))
        return false

      // Filter by search query
      const query = searchQuery().toLowerCase()
      if (query) {
        const name = addon.metadata?.name || addon.filename
        return name.toLowerCase().includes(query)
      }

      return true
    })
  })

  return {
    // Data
    allAddons, // Query with metadata (loading, error, refetch)
    addonsStore, // Reconciled addon data with stable object references
    filteredAddons, // Filtered and reconciled addon data

    // Filter states
    searchQuery,
    setSearchQuery,
    enabledAddonTypes,
    setEnabledAddonTypes,
    platformFilter,
    setPlatformFilter,

    // Table states
    ...tableState,

    // Optimistic updates
    optimisticToggleAddon,
    optimisticDeleteAddon,
    optimisticDeleteAddons,
    optimisticUpdateAddon,
    rollbackToServerState,

    // Update state management
    startUpdatingMod,
    stopUpdatingMod,
    isModUpdating,
    updatingModIds
  }
}
