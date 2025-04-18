import { DropdownMenuLabel, DropdownMenuSeparator } from "@gd/ui"
import {
  SearchApiDropdown,
  SearchCategoryDropdown,
  SearchEnvironmentDropdown,
  SearchGameVersionDropdown,
  SearchModloaderDropdown,
  SearchProjectTypeDropdown,
  SearchSortIndexDropdown,
  SearchSortOrderDropdown,
  SearchViewModeDropdown
} from "../NavSearchInputFilters"

export function SearchFiltersDropdown({
  disabled = false
}: {
  disabled?: boolean
}) {
  return (
    <>
      <DropdownMenuLabel>More Filters</DropdownMenuLabel>
      <SearchProjectTypeDropdown disabled={disabled} />
      <SearchCategoryDropdown disabled={disabled} />
      <SearchModloaderDropdown disabled={disabled} />
      <SearchGameVersionDropdown disabled={disabled} />
      <DropdownMenuSeparator />
      <DropdownMenuLabel>Environment</DropdownMenuLabel>
      <SearchEnvironmentDropdown disabled={disabled} />
      <DropdownMenuSeparator />
      <DropdownMenuLabel>Sort</DropdownMenuLabel>
      <SearchSortIndexDropdown disabled={disabled} />
      <SearchSortOrderDropdown disabled={disabled} />
      <DropdownMenuSeparator />
      <SearchViewModeDropdown disabled={disabled} />
    </>
  )
}

export { SearchApiDropdown }
