import { createSignal, createMemo, createEffect } from "solid-js"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { getInstanceImageUrl } from "@/utils/instances"

interface UseInstanceSearchOptions {
  addonType?: string
}

export const useInstanceSearch = (options?: UseInstanceSearchOptions) => {
  const [searchQuery, setSearchQuery] = createSignal("")
  const [debouncedQuery, setDebouncedQuery] = createSignal("")
  const [hoveredInstanceId, setHoveredInstanceId] = createSignal<number | null>(
    null
  )
  const globalStore = useGlobalStore()

  createEffect(() => {
    const query = searchQuery()
    const timeoutId = setTimeout(() => {
      setDebouncedQuery(query)
    }, 150)

    return () => clearTimeout(timeoutId)
  })

  const availableInstances = createMemo(() => {
    const instances = globalStore.instances.data || []
    return instances
      .filter((instance) => instance.status.status === "valid")
      .map((instance) => {
        const validInstance =
          instance.status.status === "valid" ? instance.status.value : null
        return {
          id: instance.id,
          name: instance.name,
          gameVersion: validInstance?.mc_version || "",
          modloader: validInstance?.modloader || "vanilla",
          locked: instance.locked,
          iconRevision: instance.icon_revision,
          iconUrl: instance.icon_revision
            ? getInstanceImageUrl(instance.id, instance.icon_revision)
            : null
        }
      })
      .filter((instance) => {
        // For mods, only show instances with modloaders
        if (options?.addonType === "mod") {
          return instance.modloader !== "vanilla"
        }
        // For all other addon types, show all instances
        return true
      })
  })

  const filteredInstances = createMemo(() => {
    const query = debouncedQuery().toLowerCase().trim()
    const instances = availableInstances()

    if (!query) return instances

    return instances.filter(
      (instance) =>
        instance.name.toLowerCase().includes(query) ||
        instance.gameVersion.toLowerCase().includes(query) ||
        instance.modloader.toLowerCase().includes(query)
    )
  })

  const shouldVirtualize = () => filteredInstances().length > 100

  return {
    searchQuery,
    setSearchQuery,
    debouncedQuery,
    hoveredInstanceId,
    setHoveredInstanceId,
    availableInstances,
    filteredInstances,
    shouldVirtualize
  }
}
