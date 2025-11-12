import useSearchContext from "@/components/SearchInputContext"
import { Trans } from "@gd/i18n"
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
      <div class="border-darkSlate-700/50 border-b px-6 pb-4 pt-2">
        <div class="flex flex-wrap items-center gap-2">
          <Button
            onClick={clearAllFilters}
            type="glass"
            size="small"
            class="text-xs"
          >
            <div class="i-hugeicons:cancel-circle h-4 w-4" />
            <Trans key="general:_trn_common.clear_all" />
          </Button>
          <div class="bg-darkSlate-600 h-4 w-px" />
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
