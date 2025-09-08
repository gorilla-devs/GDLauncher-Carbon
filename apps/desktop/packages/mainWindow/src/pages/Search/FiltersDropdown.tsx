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

export function FiltersDropdown(props: { disabled?: boolean }) {
  return (
    <>
      <SearchApiDropdown />
      <DropdownMenuSeparator />
      <SearchCategoryDropdown disabled={props.disabled} />
      <SearchModloaderDropdown disabled={props.disabled} />
      <SearchGameVersionDropdown disabled={props.disabled} />
      <DropdownMenuSeparator />
      <SearchEnvironmentDropdown disabled={props.disabled} />
      <DropdownMenuSeparator />
      <DropdownMenuLabel>
        <Trans key="search.sorting" />
      </DropdownMenuLabel>
      <SearchSortIndexDropdown disabled={props.disabled} />
      <SearchSortOrderDropdown disabled={props.disabled} />
    </>
  )
}

export { SearchApiDropdown }
