import useSearchContext from "@/components/SearchInputContext"
import { For, Show } from "solid-js"
import { FilterBadge } from "./FilterBadge"
import { getModloaderIcon } from "@/utils/sidebar"
import { Tooltip, TooltipContent, TooltipTrigger } from "@gd/ui"
import { shouldBypassModloaderFilter } from "@/utils/platformSearch"
import { Trans } from "@gd/i18n"

export default function ModloadersDisplay() {
  const searchContext = useSearchContext()

  const shouldHideFilters = () =>
    shouldBypassModloaderFilter(
      searchContext?.searchQuery().projectType || null
    )

  return (
    <For each={searchContext?.searchQuery().modloaders}>
      {(modloader) => {
        return (
          <Tooltip open={!shouldHideFilters() ? false : undefined}>
            <TooltipTrigger>
              <FilterBadge
                onClick={() => {
                  searchContext?.setSearchQuery((prev) => {
                    const filteredModloaders =
                      prev.modloaders?.filter((m) => m !== modloader) || []
                    return {
                      ...prev,
                      modloaders:
                        filteredModloaders.length === 0
                          ? null
                          : filteredModloaders
                    }
                  })
                }}
                class={shouldHideFilters() ? "ring-none outline-none" : ""}
              >
                <div
                  class="flex items-center gap-1"
                  classList={{
                    "text-lightSlate-900": shouldHideFilters()
                  }}
                >
                  <Show when={shouldHideFilters()}>
                    <div class="absolute left-0 top-0 flex h-full w-full items-center justify-center rounded-[5px] bg-black/20">
                      <div class="i-hugeicons:view-off-slash text-lightSlate-300 h-4 w-4" />
                    </div>
                  </Show>
                  <img class="h-4 w-4" src={getModloaderIcon(modloader)} />
                  {modloader}
                </div>
              </FilterBadge>
            </TooltipTrigger>
            <TooltipContent>
              <span class="text-xs">
                <Trans key="search:_trn_modloader_hidden" />
              </span>
            </TooltipContent>
          </Tooltip>
        )
      }}
    </For>
  )
}
