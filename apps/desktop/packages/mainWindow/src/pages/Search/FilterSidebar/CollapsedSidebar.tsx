import { Show } from "solid-js"
import useSearchContext from "@/components/SearchInputContext"
import { useGDNavigate } from "@/managers/NavigationManager"

function FilterIcon(props: {
  icon: string
  hasActive: boolean
  activeCount?: number
  onClick?: () => void
}) {
  return (
    <button
      class="hover:bg-darkSlate-700 relative flex h-10 w-10 items-center justify-center rounded-md border-none bg-transparent p-0 text-inherit transition-[colors,transform] duration-150 hover:scale-110 active:scale-95"
      classList={{ "cursor-pointer": !!props.onClick }}
      onClick={props.onClick}
    >
      <div class={`${props.icon} h-5 w-5`} />
      <Show when={props.hasActive}>
        <div class="bg-primary-500 absolute -right-0.5 -top-0.5 flex h-4 min-w-4 items-center justify-center rounded-full px-0.5 text-[9px] font-bold leading-none text-white">
          <Show
            when={props.activeCount !== undefined && props.activeCount > 0}
            fallback={<div class="h-1.5 w-1.5 rounded-full bg-white" />}
          >
            {props.activeCount}
          </Show>
        </div>
      </Show>
    </button>
  )
}

export function CollapsedSidebar(props: {
  onFilterIconClick?: (sectionId: string) => void
}) {
  const searchResults = useSearchContext()
  const navigator = useGDNavigate()

  const query = () => searchResults?.searchQuery()

  const hasContext = () =>
    !!(searchResults?.selectedInstanceId() || searchResults?.selectedServerId())

  const handleGoBack = () => {
    if (searchResults?.selectedServerId()) {
      navigator.navigate(
        `/library/server/${searchResults?.selectedServerId()}/addons`
      )
    } else if (searchResults?.selectedInstanceId()) {
      navigator.navigate(
        `/library/${searchResults?.selectedInstanceId()}/addons`
      )
    }
  }

  return (
    <div class="flex w-12 flex-col items-center gap-1 py-2">
      <Show when={hasContext()}>
        <FilterIcon
          icon="i-hugeicons:arrow-left-01"
          hasActive={false}
          onClick={handleGoBack}
        />
        <div class="bg-darkSlate-700/50 mx-1 my-1 h-px w-8" />
      </Show>
      <FilterIcon
        icon="i-hugeicons:sidebar-right"
        hasActive={false}
        onClick={() => searchResults?.setSidebarExpanded(true)}
      />
      <div class="bg-darkSlate-700/50 mx-1 my-1 h-px w-8" />
      <FilterIcon
        icon="i-hugeicons:globe-02"
        hasActive={!!query()?.searchApi}
        onClick={() => props.onFilterIconClick?.("platform")}
      />
      <FilterIcon
        icon="i-hugeicons:folder-01"
        hasActive={!!query()?.categories?.length}
        activeCount={query()?.categories?.length ?? 0}
        onClick={() => props.onFilterIconClick?.("categories")}
      />
      <FilterIcon
        icon="i-hugeicons:puzzle"
        hasActive={!!query()?.modloaders?.length}
        activeCount={query()?.modloaders?.length ?? 0}
        onClick={() => props.onFilterIconClick?.("modloaders")}
      />
      <FilterIcon
        icon="i-hugeicons:gameboy"
        hasActive={!!query()?.gameVersions?.length}
        activeCount={query()?.gameVersions?.length ?? 0}
        onClick={() => props.onFilterIconClick?.("gameVersions")}
      />
      <FilterIcon
        icon="i-hugeicons:computer"
        hasActive={!!query()?.environment}
        onClick={() => props.onFilterIconClick?.("environment")}
      />
      <Show when={query()?.searchApi}>
        <FilterIcon
          icon="i-hugeicons:sort-by-down-01"
          hasActive={!!query()?.platformFilters}
          onClick={() => props.onFilterIconClick?.("sort")}
        />
      </Show>
    </div>
  )
}
