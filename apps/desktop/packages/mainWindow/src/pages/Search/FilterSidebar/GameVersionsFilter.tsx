import { toggleGameVersion } from "@/utils/platformSearchQuery"
import { Collapsable } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { Show, createMemo } from "solid-js"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import useSearchContext from "@/components/SearchInputContext"
import { FilterWarning } from "./shared"
import { SearchableCheckboxList } from "./SearchableCheckboxList"

export function GameVersionsFilter() {
  const [t] = useTransContext()
  const globalStore = useGlobalStore()
  const searchResults = useSearchContext()

  const versions = createMemo(
    () =>
      globalStore.minecraftVersions.data?.map((version) => ({
        label: version.id,
        value: version.id
      })) || []
  )

  const selectedCount = () =>
    searchResults?.searchQuery().gameVersions?.length ?? 0

  const handleToggle = (value: string | number, checked: boolean) => {
    const stringValue = typeof value === "number" ? String(value) : value
    searchResults?.setSearchQuery((prev) =>
      toggleGameVersion(prev, stringValue, checked)
    )
  }

  return (
    <Collapsable
      title={
        <div class="flex items-center gap-2">
          <div class="i-hugeicons:gameboy h-4 w-4" />
          <Trans key="search:_trn_game_versions" />
        </div>
      }
      defaultOpened={false}
      noPadding
      count={selectedCount()}
      onClear={() => {
        searchResults?.setSearchQuery((prev) => ({
          ...prev,
          gameVersions: null
        }))
      }}
    >
      <div class="flex flex-col gap-2 px-2">
        <Show when={!!searchResults?.selectedInstanceId()}>
          <FilterWarning />
        </Show>
        <Show when={searchResults?.searchQuery().searchApi !== "modrinth"}>
          <p class="text-lightSlate-400 text-xs">
            <Trans key="search:_trn_curseforge_single_version" />
          </p>
        </Show>
        <SearchableCheckboxList
          items={versions()}
          selectedValues={() => searchResults?.searchQuery().gameVersions || []}
          onToggle={handleToggle}
          searchPlaceholder={t("search:_trn_search_versions_placeholder")}
          maxHeight={250}
          emptyMessage={<Trans key="content:_trn_common.no_versions_found" />}
        />
      </div>
    </Collapsable>
  )
}
