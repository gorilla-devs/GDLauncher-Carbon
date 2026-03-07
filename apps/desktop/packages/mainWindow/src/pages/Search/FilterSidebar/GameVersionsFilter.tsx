import { Collapsable } from "@gd/ui"
import { Trans } from "@gd/i18n"
import { Show, createMemo } from "solid-js"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import useSearchContext from "@/components/SearchInputContext"
import { FilterWarning } from "./shared"
import { SearchableCheckboxList } from "./SearchableCheckboxList"

export function GameVersionsFilter() {
  const globalStore = useGlobalStore()
  const searchResults = useSearchContext()

  const versions = createMemo(
    () =>
      globalStore.minecraftVersions.data?.map((version) => ({
        label: version.id,
        value: version.id
      })) || []
  )

  const handleToggle = (value: string, checked: boolean) => {
    searchResults?.setSearchQuery((prev) => {
      const prevGameVersions = prev.gameVersions || []
      if (checked) {
        if (!prevGameVersions.includes(value)) {
          return {
            ...prev,
            gameVersions: [...prevGameVersions, value]
          }
        }
      } else {
        const filtered = prevGameVersions.filter((v) => v !== value)
        return {
          ...prev,
          gameVersions: filtered.length === 0 ? null : filtered
        }
      }
      return prev
    })
  }

  return (
    <Collapsable
      title={<div class="flex items-center gap-2"><div class="i-hugeicons:gameboy h-4 w-4" /><Trans key="search:_trn_game_versions" /></div>}
      defaultOpened={false}
      noPadding
    >
      <div class="flex flex-col gap-1 px-2">
        <Show when={!!searchResults?.selectedInstanceId()}>
          <FilterWarning />
        </Show>
        <SearchableCheckboxList
          items={versions()}
          selectedValues={() => searchResults?.searchQuery().gameVersions || []}
          onToggle={handleToggle}
          searchPlaceholder="Search versions..."
          maxHeight={250}
          emptyMessage={<Trans key="content:_trn_common.no_versions_found" />}
        />
      </div>
    </Collapsable>
  )
}
