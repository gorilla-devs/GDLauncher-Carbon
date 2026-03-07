import { Collapsable, Radio } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { Show, Switch, Match } from "solid-js"
import useSearchContext from "@/components/SearchInputContext"
import { capitalize } from "@/utils/helpers"
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg"
import CurseforgeLogo from "/assets/images/icons/curseforge_logo.svg"

function CurseforgeSortFilter() {
  const searchResults = useSearchContext()

  const sortFieldOptions = [
    { value: "featured", key: "search:_trn_featured" },
    { value: "popularity", key: "search:_trn_popularity" },
    { value: "totalDownloads", key: "search:_trn_downloads" },
    { value: "lastUpdated", key: "search:_trn_last_updated" },
    { value: "name", key: "search:_trn_name" },
    { value: "author", key: "search:_trn_author" }
  ] as const

  const orderOptions = ["ascending", "descending"] as const

  const currentFilters = () => {
    const filters = searchResults?.searchQuery().platformFilters
    if (filters?.platform === "curseforge") {
      return filters.filters
    }
    return { sort_field: null, sort_order: null }
  }

  const updateFilters = (
    updates: Partial<ReturnType<typeof currentFilters>>
  ) => {
    searchResults?.setSearchQuery((prev) => ({
      ...prev,
      platformFilters: {
        platform: "curseforge" as const,
        filters: {
          ...currentFilters(),
          ...updates
        }
      }
    }))
  }

  return (
    <div class="flex flex-col gap-1">
      <Collapsable
        title={<Trans key="search:_trn_sort_by" />}
        size="small"
        defaultOpened
        noPadding
      >
        <div class="flex flex-col">
          {sortFieldOptions.map((option) => (
            <Radio
              value={option.value}
              checked={currentFilters().sort_field === option.value}
              onChange={() => {
                if (option.value === currentFilters().sort_field) {
                  updateFilters({ sort_field: null })
                } else {
                  updateFilters({ sort_field: option.value })
                }
              }}
            >
              <span class="text-sm">
                <Trans key={option.key} />
              </span>
            </Radio>
          ))}
        </div>
      </Collapsable>

      <Collapsable
        title={<Trans key="search:_trn_order" />}
        size="small"
        defaultOpened
        noPadding
      >
        <div class="flex flex-col">
          {orderOptions.map((value) => (
            <Radio
              value={value}
              checked={currentFilters().sort_order === value}
              onChange={() => {
                if (value === currentFilters().sort_order) {
                  updateFilters({ sort_order: null })
                } else {
                  updateFilters({ sort_order: value })
                }
              }}
            >
              <span class="text-sm">{capitalize(value)}</span>
            </Radio>
          ))}
        </div>
      </Collapsable>
    </div>
  )
}

function ModrinthSortFilter() {
  const searchResults = useSearchContext()

  const sortOptions = [
    { value: "relevance", key: "search:_trn_relevance" },
    { value: "downloads", key: "search:_trn_downloads" },
    { value: "follows", key: "search:_trn_follows" },
    { value: "newest", key: "search:_trn_newest" },
    { value: "updated", key: "search:_trn_updated" }
  ] as const

  const currentFilters = () => {
    const filters = searchResults?.searchQuery().platformFilters
    if (filters?.platform === "modrinth") {
      return filters.filters
    }
    return { sort_index: null }
  }

  const updateFilters = (
    updates: Partial<ReturnType<typeof currentFilters>>
  ) => {
    searchResults?.setSearchQuery((prev) => ({
      ...prev,
      platformFilters: {
        platform: "modrinth" as const,
        filters: {
          ...currentFilters(),
          ...updates
        }
      }
    }))
  }

  return (
    <div class="flex flex-col">
      {sortOptions.map((option) => (
        <Radio
          value={option.value}
          checked={currentFilters().sort_index === option.value}
          onChange={() => {
            if (option.value === currentFilters().sort_index) {
              updateFilters({ sort_index: null })
            } else {
              updateFilters({ sort_index: option.value })
            }
          }}
        >
          <span class="text-sm">
            <Trans key={option.key} />
          </span>
        </Radio>
      ))}
    </div>
  )
}

export function SortFilter() {
  const searchResults = useSearchContext()
  const [t] = useTransContext()
  const selectedApi = () => searchResults?.searchQuery().searchApi

  return (
    <Show when={selectedApi()}>
      <Collapsable
        title={
          <div class="flex items-center gap-2">
            <div class="i-hugeicons:sort-by-down-01 h-4 w-4" />
            <span>
              {t("search:_trn_sort_by")}
            </span>
          </div>
        }
        defaultOpened
        noPadding
      >
        <Switch>
          <Match when={selectedApi() === "curseforge"}>
            <CurseforgeSortFilter />
          </Match>
          <Match when={selectedApi() === "modrinth"}>
            <ModrinthSortFilter />
          </Match>
        </Switch>
      </Collapsable>
    </Show>
  )
}
