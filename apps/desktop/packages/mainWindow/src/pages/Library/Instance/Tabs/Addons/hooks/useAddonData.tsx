import { createSignal, createMemo, createEffect } from "solid-js"
import { createStore, reconcile } from "solid-js/store"
import { useParams } from "@solidjs/router"
import { rspc } from "@/utils/rspcClient"
import { AddonType, Mod } from "@gd/core_module/bindings"
import {
  SortingState,
  ColumnFiltersState,
  VisibilityState,
  RowSelectionState
} from "@tanstack/solid-table"

export const useAddonData = () => {
  const params = useParams()

  // Filter states
  const [searchQuery, setSearchQuery] = createSignal("")
  const [enabledAddonTypes, setEnabledAddonTypes] = createStore<
    Record<AddonType, boolean>
  >({
    mods: true,
    shaders: true,
    resourcepacks: true,
    datapacks: true,
    worlds: true
  })
  const [platformFilter, setPlatformFilter] = createSignal<
    "all" | "curseforge" | "modrinth" | "local"
  >("all")

  // Table states
  const [sorting, setSorting] = createSignal<SortingState>([])
  const [columnFilters, setColumnFilters] = createSignal<ColumnFiltersState>([])
  const [columnVisibility, setColumnVisibility] = createSignal<VisibilityState>(
    {}
  )
  const [rowSelection, setRowSelection] = createSignal<RowSelectionState>({})

  // Reconciled store for addons to maintain stable object references
  const [addonsStore, setAddonsStore] = createStore<Mod[]>([])

  // Fetch all addons at once
  const allAddons = rspc.createQuery(() => ({
    queryKey: [
      "instance.getInstanceMods",
      {
        instance_id: parseInt(params.id, 10),
        addon_type: null
      }
    ]
  }))

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
    const filteredAddons = addonsStore.filter((addon) => addon.id !== addonId)
    setAddonsStore(reconcile(filteredAddons, { key: "id" }))
  }

  const optimisticDeleteAddons = (addonIds: string[]) => {
    const filteredAddons = addonsStore.filter(
      (addon) => !addonIds.includes(addon.id)
    )
    setAddonsStore(reconcile(filteredAddons, { key: "id" }))
  }

  const rollbackToServerState = () => {
    if (allAddons.data) {
      setAddonsStore(reconcile(allAddons.data, { key: "id" }))
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
    setEnabledAddonTypes: (type: AddonType, enabled: boolean) => {
      setEnabledAddonTypes(type, enabled)
    },
    platformFilter,
    setPlatformFilter,

    // Table states
    sorting,
    setSorting,
    columnFilters,
    setColumnFilters,
    columnVisibility,
    setColumnVisibility,
    rowSelection,
    setRowSelection,

    // Optimistic updates
    optimisticToggleAddon,
    optimisticDeleteAddon,
    optimisticDeleteAddons,
    rollbackToServerState
  }
}
