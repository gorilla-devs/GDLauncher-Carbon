import { Show } from "solid-js"
import useSearchContext from "@/components/SearchInputContext"
import { useTransContext } from "@gd/i18n"
import { Tooltip, TooltipContent, TooltipTrigger } from "@gd/ui"

function FilterIcon(props: {
  icon: string
  label: string
  hasActive: boolean
  onClick?: () => void
}) {
  return (
    <Tooltip>
      <TooltipTrigger
        class="hover:bg-darkSlate-600 relative flex h-10 w-10 items-center justify-center rounded-md transition-[colors,transform] duration-150 hover:scale-110 active:scale-95"
        classList={{ "cursor-pointer": !!props.onClick }}
        onClick={props.onClick}
      >
        <div class={`${props.icon} h-5 w-5`} />
        <Show when={props.hasActive}>
          <div class="bg-primary-500 absolute right-1 top-1 h-2 w-2 rounded-full" />
        </Show>
      </TooltipTrigger>
      <TooltipContent>{props.label}</TooltipContent>
    </Tooltip>
  )
}

export function CollapsedSidebar(props: {
  onFilterIconClick?: (sectionId: string) => void
}) {
  const searchResults = useSearchContext()
  const [t] = useTransContext()

  const query = () => searchResults?.searchQuery()

  return (
    <div class="flex w-12 flex-col items-center gap-1 py-2">
      <FilterIcon
        icon="i-hugeicons:sidebar-right"
        label={t("search:_trn_filters")}
        hasActive={false}
        onClick={() => searchResults?.setSidebarExpanded(true)}
      />
      <div class="bg-darkSlate-700/50 mx-1 my-1 h-px w-8" />
      <FilterIcon
        icon="i-hugeicons:globe-02"
        label={t("search:_trn_platform")}
        hasActive={!!query()?.searchApi}
        onClick={() => props.onFilterIconClick?.("platform")}
      />
      <FilterIcon
        icon="i-hugeicons:folder-01"
        label={t("search:_trn_categories")}
        hasActive={!!(query()?.categories?.length)}
        onClick={() => props.onFilterIconClick?.("categories")}
      />
      <FilterIcon
        icon="i-hugeicons:puzzle"
        label={t("search:_trn_modloaders")}
        hasActive={!!(query()?.modloaders?.length)}
        onClick={() => props.onFilterIconClick?.("modloaders")}
      />
      <FilterIcon
        icon="i-hugeicons:gameboy"
        label={t("search:_trn_game_versions")}
        hasActive={!!(query()?.gameVersions?.length)}
        onClick={() => props.onFilterIconClick?.("gameVersions")}
      />
      <FilterIcon
        icon="i-hugeicons:computer"
        label={t("search:_trn_environment")}
        hasActive={!!query()?.environment}
        onClick={() => props.onFilterIconClick?.("environment")}
      />
      <Show when={query()?.searchApi}>
        <FilterIcon
          icon="i-hugeicons:sort-by-down-01"
          label={t("search:_trn_sort_by")}
          hasActive={!!query()?.platformFilters}
          onClick={() => props.onFilterIconClick?.("sort")}
        />
      </Show>
    </div>
  )
}
