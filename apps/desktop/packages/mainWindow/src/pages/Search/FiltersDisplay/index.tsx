import useSearchContext from "@/components/SearchInputContext"
import { Show } from "solid-js"
import { Button } from "@gd/ui"
import InstanceDisplay from "./InstanceDisplay"
import CategoriesDisplay from "./CategoriesDisplay"
import ModloadersDisplay from "./ModloadersDisplay"
import GameVersionsDisplay from "./GameVersionsDisplay"
import SearchApiDisplay from "./SearchApiDisplay"
import EnvironmentDisplay from "./EnvironmentDisplay"
import SortDisplay from "./SortDisplay"

export default function FiltersDisplay() {
  const searchContext = useSearchContext()

  const hasActiveFilters = () => {
    const query = searchContext?.searchQuery()
    return !!(
      query?.categories?.length ||
      query?.modloaders?.length ||
      query?.gameVersions?.length ||
      query?.searchApi ||
      query?.environment ||
      query?.platformFilters ||
      searchContext?.selectedInstanceId()
    )
  }

  const clearAllFilters = () => {
    searchContext?.setSearchQuery((prev) => ({
      ...prev,
      categories: null,
      modloaders: null,
      gameVersions: null,
      searchApi: null,
      environment: null,
      sortIndex: null,
      sortOrder: null
    }))
    searchContext?.setSelectedInstanceId(undefined)
  }

  return (
    <Show when={hasActiveFilters()}>
      <div class="px-6 pt-2 pb-4 border-b border-darkSlate-700/50">
        <div class="flex items-center gap-2 flex-wrap">
          <Button
            onClick={clearAllFilters}
            type="glass"
            size="small"
            class="text-xs"
          >
            <div class="i-ri:close-circle-line h-4 w-4" />
            Clear all
          </Button>
          <div class="w-px h-4 bg-darkSlate-600" />
          <InstanceDisplay />
          <SearchApiDisplay />
          <CategoriesDisplay />
          <ModloadersDisplay />
          <GameVersionsDisplay />
          <EnvironmentDisplay />
          <SortDisplay />
        </div>
      </div>
    </Show>
  )
}
