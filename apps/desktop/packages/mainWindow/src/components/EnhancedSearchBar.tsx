import { Show, createEffect, createSignal } from "solid-js"
import { AddonTypeDropdown } from "./AddonTypeDropdown"
import useSearchContext from "./SearchInputContext"
import { useGDNavigate } from "@/managers/NavigationManager"
import { useMatch } from "@solidjs/router"
import { useTransContext } from "@gd/i18n"
import { DropdownMenu, DropdownMenuContent, DropdownMenuTrigger } from "@gd/ui"
import { FiltersDropdown } from "@/pages/Search/FiltersDropdown"

export function EnhancedSearchBar() {
  const searchResults = useSearchContext()
  const navigator = useGDNavigate()
  const isSearchPage = useMatch(() => "/search/*")
  const isAddonPage = useMatch(() => "/addon/*")
  const [t] = useTransContext()

  // Optimistic expansion state for instant feedback
  const [optimisticExpand, setOptimisticExpand] = createSignal(false)

  // Keep expanded on search pages and addon pages
  const isExpanded = () =>
    !!(isSearchPage() || isAddonPage() || optimisticExpand())

  let simpleInputRef: HTMLInputElement | undefined
  let expandedInputRef: HTMLInputElement | undefined

  const handleSimpleClick = () => {
    if (!isExpanded()) {
      setOptimisticExpand(true)
      navigator.navigate("/search")
      setTimeout(() => setOptimisticExpand(false), 350)
    }
  }

  createEffect(() => {
    if (isSearchPage() && expandedInputRef) {
      setTimeout(() => {
        expandedInputRef?.focus()
      }, 0)
    }
  })

  return (
    <div
      class="bg-darkSlate-700 outline-2 outline outline-offset-2 outline-transparent has-[:focus-visible]:outline-darkSlate-500 hover:outline-darkSlate-600 hover:has-[:focus-visible]:outline-darkSlate-500 flex h-10 items-center gap-2 overflow-hidden rounded-md duration-300"
      style={{
        transition:
          "width 300ms cubic-bezier(0.4, 0, 0.2, 1), padding 300ms cubic-bezier(0.4, 0, 0.2, 1), opacity 300ms cubic-bezier(0.4, 0, 0.2, 1), outline-color 150ms cubic-bezier(0.4, 0, 0.2, 1)",
        "will-change": "width, padding",
        contain: "layout",
        transform: "translateZ(0)"
      }}
      classList={{
        "w-80 px-4": !isExpanded(),
        "w-full max-w-[600px] px-2": isExpanded()
      }}
    >
      <div
        class="overflow-hidden transition-[opacity,max-width] duration-300 ease-[cubic-bezier(.4,0,.2,1)]"
        classList={{
          "opacity-0 pointer-events-none max-w-0": !isExpanded(),
          "opacity-100 max-w-[200px] delay-[40ms]": isExpanded()
        }}
      >
        <AddonTypeDropdown />
      </div>

      <div
        class="bg-darkSlate-500 h-6 transition-[opacity,width] duration-300 ease-[cubic-bezier(.4,0,.2,1)]"
        classList={{
          "opacity-0 w-0": !isExpanded(),
          "opacity-100 w-px delay-[75ms]": isExpanded()
        }}
      />

      <div class="i-hugeicons:search-01 text-darkSlate-400 h-5 w-5 shrink-0" />

      <Show when={!isExpanded()}>
        <input
          ref={simpleInputRef}
          placeholder={t("search:_trn_search_discover_anything")}
          class="placeholder:text-darkSlate-400 text-lightSlate-50 h-full flex-1 cursor-pointer bg-transparent text-sm outline-none"
          value=""
          readOnly
          onClick={handleSimpleClick}
        />
      </Show>

      <Show when={isExpanded()}>
        <input
          ref={expandedInputRef}
          placeholder={t("search:_trn_search_discover_anything")}
          class="placeholder:text-darkSlate-400 text-lightSlate-50 h-full flex-1 bg-transparent text-sm outline-none"
          value={searchResults?.searchQuery().searchQuery ?? ""}
          onInput={(e) => {
            searchResults?.setSearchQuery((prev) => ({
              ...prev,
              searchQuery: e.target.value
            }))
          }}
        />
      </Show>

      <Show
        when={
          isExpanded() &&
          (searchResults?.searchQuery().searchQuery?.length || 0 > 0)
        }
      >
        <div
          class="i-hugeicons:cancel-01 text-lg text-darkSlate-500 cursor-pointer transition-colors duration-200 ease-in-out hover:text-white"
          onClick={(e) => {
            e.preventDefault()
            e.stopPropagation()
            searchResults?.setSearchQuery((prev) => ({
              ...prev,
              searchQuery: ""
            }))
          }}
        />
      </Show>

      <div
        class="bg-darkSlate-500 h-6 transition-[opacity,width] duration-300 ease-[cubic-bezier(.4,0,.2,1)]"
        classList={{
          "opacity-0 w-0": !isExpanded(),
          "opacity-100 w-px delay-[75ms]": isExpanded()
        }}
      />

      <div
        class="overflow-hidden transition-[opacity,max-width] duration-300 ease-[cubic-bezier(.4,0,.2,1)]"
        classList={{
          "opacity-0 pointer-events-none max-w-0": !isExpanded(),
          "opacity-100 max-w-[200px] delay-[110ms]": isExpanded()
        }}
      >
        <DropdownMenu>
          <DropdownMenuTrigger
            class="text-lightSlate-50 hover:bg-darkSlate-600 data-[expanded]:bg-darkSlate-600 flex items-center justify-center rounded p-1.5 transition-colors hover:text-white data-[expanded]:text-white"
            title={t("search:_trn_filters")}
          >
            <div class="i-hugeicons:filter text-lg" />
          </DropdownMenuTrigger>
          <DropdownMenuContent>
            <FiltersDropdown />
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </div>
  )
}
