import { DropdownMenuLabel, DropdownMenuSeparator } from "@gd/ui"
import {
  SearchApiDropdown,
  SearchCategoryDropdown,
  SearchEnvironmentDropdown,
  SearchGameVersionDropdown,
  SearchModloaderDropdown,
  SearchSortIndexDropdown,
  SearchSortOrderDropdown,
  SearchViewModeDropdown
} from "@/components/NavSearchInputFilters"

export function FiltersDropdown(props: { disabled?: boolean }) {
  return (
    <>
      <DropdownMenuLabel>More Filters</DropdownMenuLabel>
      <SearchCategoryDropdown disabled={props.disabled} />
      <SearchModloaderDropdown disabled={props.disabled} />
      <SearchGameVersionDropdown disabled={props.disabled} />
      <DropdownMenuSeparator />
      <DropdownMenuLabel>Environment</DropdownMenuLabel>
      <SearchEnvironmentDropdown disabled={props.disabled} />
      <DropdownMenuSeparator />
      <DropdownMenuLabel>Sort</DropdownMenuLabel>
      <SearchSortIndexDropdown disabled={props.disabled} />
      <SearchSortOrderDropdown disabled={props.disabled} />
      <DropdownMenuSeparator />
      <SearchViewModeDropdown disabled={props.disabled} />
    </>
  )
}

export { SearchApiDropdown }
