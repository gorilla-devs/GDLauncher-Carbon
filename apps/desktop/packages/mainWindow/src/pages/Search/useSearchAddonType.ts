import { createEffect, createMemo } from "solid-js"
import { useLocation, useParams } from "@solidjs/router"
import { useGDNavigate } from "@/managers/NavigationManager"
import useSearchContext from "@/components/SearchInputContext"
import { resolveBrowsableAddonType } from "@/utils/browsableAddonType"

export function useSearchAddonType(
  searchContext: ReturnType<typeof useSearchContext>
) {
  const params = useParams<{ type?: string }>()
  const location = useLocation()
  const navigator = useGDNavigate()

  const type = createMemo(() => {
    const explicit = resolveBrowsableAddonType(params.type)
    if (explicit) return explicit
    const fallback =
      resolveBrowsableAddonType(searchContext?.defaultAddonType()) ?? "modpack"
    // A target entry uses that target's current default; standalone browsing
    // can resume the previous valid query. Metadata may resolve after mount.
    if (
      searchContext?.selectedInstanceId() ||
      searchContext?.selectedServerId()
    )
      return fallback
    return (
      resolveBrowsableAddonType(searchContext?.searchQuery().projectType) ??
      fallback
    )
  })

  createEffect(() => {
    const resolved = type()
    if (resolved !== searchContext?.searchQuery().projectType) {
      searchContext?.setSearchQuery((prev) => ({
        ...prev,
        projectType: resolved
      }))
    }
  })

  createEffect(() => {
    const routeType = params.type
    const resolved = type()
    // Existing aliases and unsupported segments visibly resolve to the same
    // type used by filters and requests. Missing segments stay reactive.
    if (routeType && routeType !== resolved) {
      navigator.navigate(`/search/${resolved}${location.search}`, {
        replace: true
      })
    }
  })

  return type
}
