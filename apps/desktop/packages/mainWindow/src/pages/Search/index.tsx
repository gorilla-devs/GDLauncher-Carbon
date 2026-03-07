import { JSX, Show } from "solid-js"
import { FilterSidebar } from "./FilterSidebar"
import useSearchContext from "@/components/SearchInputContext"
import { useGDNavigate } from "@/managers/NavigationManager"
import { Trans } from "@gd/i18n"
import { Button } from "@gd/ui"

export function Search(props: { children?: JSX.Element }) {
  const searchContext = useSearchContext()
  const navigator = useGDNavigate()

  return (
    <div class="text-lightSlate-50 bg-darkSlate-700 h-content box-border flex max-h-full min-h-full w-full flex-1 justify-center overflow-hidden pr-4 pt-4">
      <div
        id="gdl-content-wrapper"
        class="bg-darkSlate-800 relative box-border flex h-auto w-full flex-1 flex-col overflow-auto rounded-r-2xl rounded-br-none"
        style={{
          "scrollbar-gutter": "stable"
        }}
      >
        <div class="flex h-full flex-1 overflow-hidden">
          <FilterSidebar />
          <div class="flex h-full flex-1 flex-col overflow-hidden">
            <Show when={searchContext?.selectedInstanceId()}>
              <div class="border-darkSlate-700/50 flex items-center border-b px-6 py-2">
                <Button
                  size="small"
                  type="secondary"
                  onClick={() => {
                    navigator.navigate(
                      `/library/${searchContext?.selectedInstanceId()}/addons`
                    )
                  }}
                >
                  <div class="i-hugeicons:arrow-left-01" />
                  <Trans key="search:_trn_go_back" />
                </Button>
              </div>
            </Show>
            {props.children}
          </div>
        </div>
      </div>
    </div>
  )
}

export default Search
