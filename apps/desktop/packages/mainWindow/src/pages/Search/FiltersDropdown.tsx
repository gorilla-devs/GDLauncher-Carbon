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
import { Trans } from "@gd/i18n"

export function FiltersDropdown(props: { disabled?: boolean }) {
  return (
    <>
      <DropdownMenuLabel>
        <Trans key="search.more_filters" />
      </DropdownMenuLabel>
      <SearchCategoryDropdown disabled={props.disabled} />
      <SearchModloaderDropdown disabled={props.disabled} />
      <SearchGameVersionDropdown disabled={props.disabled} />
      <DropdownMenuSeparator />
      <DropdownMenuLabel>
        <Trans key="search.environment" />
      </DropdownMenuLabel>
      <SearchEnvironmentDropdown disabled={props.disabled} />
      <DropdownMenuSeparator />
      <DropdownMenuLabel>
        <Trans key="ui.sort_options" />
      </DropdownMenuLabel>
      <SearchSortIndexDropdown disabled={props.disabled} />
      <SearchSortOrderDropdown disabled={props.disabled} />
      <DropdownMenuSeparator />
      <SearchViewModeDropdown disabled={props.disabled} />
    </>
  )
}

export { SearchApiDropdown }
