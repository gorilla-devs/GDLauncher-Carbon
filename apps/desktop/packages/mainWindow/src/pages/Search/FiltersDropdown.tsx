import { DropdownMenuLabel, DropdownMenuSeparator } from "@gd/ui"
import {
  SearchApiDropdown,
  SearchCategoryDropdown,
  SearchEnvironmentDropdown,
  SearchGameVersionDropdown,
  SearchModloaderDropdown,
  SearchSortIndexDropdown,
  SearchSortOrderDropdown
} from "@/components/NavSearchInputFilters"
import { Trans } from "@gd/i18n"

export function FiltersDropdown() {
  return (
    <>
      <SearchApiDropdown />
      <DropdownMenuSeparator />
      <SearchCategoryDropdown />
      <SearchModloaderDropdown />
      <SearchGameVersionDropdown />
      <DropdownMenuSeparator />
      <SearchEnvironmentDropdown />
      <DropdownMenuSeparator />
      <DropdownMenuLabel>
        <Trans key="search.sorting" />
      </DropdownMenuLabel>
      <SearchSortIndexDropdown />
      <SearchSortOrderDropdown />
    </>
  )
}

export { SearchApiDropdown }
