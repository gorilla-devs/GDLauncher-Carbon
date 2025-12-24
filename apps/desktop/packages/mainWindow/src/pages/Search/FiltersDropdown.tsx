import { DropdownMenuLabel, DropdownMenuSeparator } from "@gd/ui"
import {
  SearchApiDropdown,
  SearchCategoryDropdown,
  SearchEnvironmentDropdown,
  SearchGameVersionDropdown,
  SearchModloaderDropdown,
  PlatformSpecificFilters
} from "@/components/NavSearchInputFilters"
import { Trans } from "@gd/i18n"
import useSearchContext from "@/components/SearchInputContext"
import { ViewModeToggle } from "./ViewModeToggle"

export function FiltersDropdown() {
  const searchContext = useSearchContext()

  const resetAllFilters = () => {
    searchContext?.setSearchQuery((prev) => ({
      ...prev,
      categories: null,
      gameVersions: null,
      modloaders: null,
      environment: null,
      platformFilters: null,
      searchApi: null
    }))
  }

  return (
    <>
      <DropdownMenuLabel>
        <div class="flex items-center justify-between gap-2">
          <Trans key="search:_trn_view_mode" />
          <ViewModeToggle />
        </div>
      </DropdownMenuLabel>
      <DropdownMenuSeparator />
      <DropdownMenuLabel>
        <div class="flex items-center justify-between gap-2">
          <div>
            <Trans key="search:_trn_filters" />
          </div>
          <div
            class="text-lightSlate-900 hover:text-lightSlate-50 cursor-pointer text-xs transition-colors duration-200 ease-[cubic-bezier(.4,0,.2,1)]"
            onClick={resetAllFilters}
          >
            <Trans key="search:_trn_clear_all_filters" />
          </div>
        </div>
      </DropdownMenuLabel>
      <DropdownMenuSeparator />
      <SearchApiDropdown />
      <DropdownMenuSeparator />
      <DropdownMenuLabel>
        <Trans key="search:_trn_universal_filters" />
      </DropdownMenuLabel>
      <SearchCategoryDropdown />
      <SearchModloaderDropdown />
      <SearchGameVersionDropdown />
      <DropdownMenuSeparator />
      <SearchEnvironmentDropdown />
      <PlatformSpecificFilters />
    </>
  )
}

export { SearchApiDropdown }
