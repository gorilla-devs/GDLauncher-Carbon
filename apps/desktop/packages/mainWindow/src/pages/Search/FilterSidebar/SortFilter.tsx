import { Collapsable, Radio } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { Show, Switch, Match, For } from "solid-js"
import useSearchContext from "@/components/SearchInputContext"
import { capitalize } from "@/utils/helpers"

const CF_SORT_FIELDS = [
  { value: "featured", key: "search:_trn_featured" },
  { value: "popularity", key: "search:_trn_popularity" },
  { value: "totalDownloads", key: "search:_trn_downloads" },
  { value: "lastUpdated", key: "search:_trn_last_updated" },
  { value: "name", key: "search:_trn_name" },
  { value: "author", key: "search:_trn_author" }
] as const

const CF_ORDER_OPTIONS = ["ascending", "descending"] as const

const MR_SORT_OPTIONS = [
  { value: "relevance", key: "search:_trn_relevance" },
  { value: "downloads", key: "search:_trn_downloads" },
  { value: "follows", key: "search:_trn_follows" },
  { value: "newest", key: "search:_trn_newest" },
  { value: "updated", key: "search:_trn_updated" }
] as const

function CurseforgeSortFilter() {
  const searchResults = useSearchContext()

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
          <For each={CF_SORT_FIELDS}>
            {(option) => (
              <Radio
                value={option.value}
                checked={currentFilters().sort_field === option.value}
                allowDeselect
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
            )}
          </For>
        </div>
      </Collapsable>

      <Collapsable
        title={<Trans key="search:_trn_order" />}
        size="small"
        defaultOpened
        noPadding
      >
        <div class="flex flex-col">
          <For each={[...CF_ORDER_OPTIONS]}>
            {(value) => (
              <Radio
                value={value}
                checked={currentFilters().sort_order === value}
                allowDeselect
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
            )}
          </For>
        </div>
      </Collapsable>
    </div>
  )
}

function ModrinthSortFilter() {
  const searchResults = useSearchContext()

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
      <For each={MR_SORT_OPTIONS}>
        {(option) => (
          <Radio
            value={option.value}
            checked={currentFilters().sort_index === option.value}
            allowDeselect
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
        )}
      </For>
    </div>
  )
}

export function SortFilter() {
  const searchResults = useSearchContext()
  const [t] = useTransContext()
  const selectedApi = () => searchResults?.searchQuery().searchApi

  const hasActiveSort = () => !!searchResults?.searchQuery().platformFilters

  return (
    <Show when={selectedApi()}>
      <Collapsable
        title={
          <div class="flex items-center gap-2">
            <div class="i-hugeicons:sort-by-down-01 h-4 w-4" />
            <span>{t("search:_trn_sort_by")}</span>
          </div>
        }
        defaultOpened
        noPadding
        count={hasActiveSort() ? 1 : 0}
        onClear={() => {
          searchResults?.setSearchQuery((prev) => ({
            ...prev,
            platformFilters: null
          }))
        }}
      >
        <div class="px-2">
          <Switch>
            <Match when={selectedApi() === "curseforge"}>
              <CurseforgeSortFilter />
            </Match>
            <Match when={selectedApi() === "modrinth"}>
              <ModrinthSortFilter />
            </Match>
          </Switch>
        </div>
      </Collapsable>
    </Show>
  )
}
