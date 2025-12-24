import { Show } from "solid-js"
import { useMatch } from "@solidjs/router"
import { InlineFilterBadges } from "./InlineFilterBadges"
import useSearchContext from "./SearchInputContext"
import { Trans, useTransContext } from "@gd/i18n"
import { Button } from "@gd/ui"
import { useGDNavigate } from "@/managers/NavigationManager"

export function FilterBadgesBar() {
  const isSearchPage = useMatch(() => "/search/*")
  const isAddonViewPage = useMatch(() => "/addon/*/*")
  const searchContext = useSearchContext()
  const [t] = useTransContext()
  const navigator = useGDNavigate()

  const isExpanded = () => !!isSearchPage() || !!isAddonViewPage()

  const hasActiveFilters = () => {
    const query = searchContext?.searchQuery()
    return !!(
      query?.categories?.length ||
      query?.modloaders?.length ||
      query?.gameVersions?.length ||
      query?.searchApi ||
      query?.environment ||
      query?.platformFilters
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
      platformFilters: null
    }))
  }

  return (
    <Show when={isExpanded() && hasActiveFilters()}>
      <div
        class="bg-darkSlate-800 border-darkSlate-700 border-b transition-all duration-300 ease-spring"
        style={{ "view-transition-name": "filter-badges-bar" }}
      >
        <div class="flex items-center gap-4 px-6 py-3">
          <Show when={searchContext?.selectedInstanceId()}>
            <Button
              size="small"
              type="outline"
              onClick={() => {
                navigator.navigate(
                  `/library/${searchContext?.selectedInstanceId()}/addons`
                )
              }}
            >
              <div class="i-hugeicons:arrow-left-01" />
              <Trans key="search:_trn_go_back" />
            </Button>
          </Show>
          <div class="flex flex-1 items-center justify-center">
            <div class="scrollbar-hide flex max-w-[700px] items-center gap-2 overflow-visible">
              <button
                class="text-lightSlate-400 hover:text-lightSlate-50 hover:bg-darkSlate-700 flex shrink-0 items-center gap-1.5 rounded px-2.5 py-1 text-xs font-medium transition-colors"
                onClick={clearAllFilters}
              >
                <div class="i-hugeicons:delete-02 text-sm" />
                {t("search:_trn_clear_all_filters")}
              </button>
              <div class="bg-darkSlate-600 h-4 w-px shrink-0" />
              <InlineFilterBadges />
            </div>
          </div>
        </div>
      </div>
    </Show>
  )
}
