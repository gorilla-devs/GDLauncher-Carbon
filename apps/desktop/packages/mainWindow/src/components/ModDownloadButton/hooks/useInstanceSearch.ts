import { createSignal, createMemo, createEffect, onCleanup } from "solid-js"
import type { FEUnifiedSearchResult } from "@gd/core_module/bindings"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import useSearchContext from "@/components/SearchInputContext"
import { getInstanceImageUrl } from "@/utils/instances"
import { asksWhichWorld } from "@/utils/worldTarget"

interface UseInstanceSearchOptions {
  /** The project a picked instance gets. */
  addon?: () => Pick<FEUnifiedSearchResult, "type" | "loaders"> | undefined
}

export const useInstanceSearch = (options?: UseInstanceSearchOptions) => {
  const [searchQuery, setSearchQuery] = createSignal("")
  const [debouncedQuery, setDebouncedQuery] = createSignal("")
  const [hoveredInstanceId, setHoveredInstanceId] = createSignal<number | null>(
    null
  )
  const globalStore = useGlobalStore()
  const searchContext = useSearchContext()

  createEffect(() => {
    const query = searchQuery()
    const timeoutId = setTimeout(() => {
      setDebouncedQuery(query)
    }, 150)

    onCleanup(() => clearTimeout(timeoutId))
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
        const addon = options?.addon?.()
        // For mods, only show instances with modloaders. A data pack Modrinth
        // types as a mod goes into a world, which any instance can have: the
        // pick then asks which one.
        if (
          addon?.type === "mod" &&
          !asksWhichWorld(addon, searchContext?.searchQuery().projectType)
        ) {
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
