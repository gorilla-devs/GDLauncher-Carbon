import { Show, createEffect, createSignal } from "solid-js"
import { AddonTypeDropdown } from "./AddonTypeDropdown"
import useSearchContext from "./SearchInputContext"
import { useGDNavigate } from "@/managers/NavigationManager"
import { useMatch } from "@solidjs/router"
import { Trans, useTransContext } from "@gd/i18n"
import { OnboardingTip } from "@/components/Onboarding"
import { Popover, PopoverContent, PopoverTrigger } from "@gd/ui"

const SearchSyntaxTips = () => (
  <div class="space-y-2 text-sm">
    <p>
      <Trans key="onboarding:_trn_search_syntax_description" />
    </p>
    <ul class="list-disc space-y-1 pl-4">
      <li>
        <strong>
          <Trans key="general:_trn_text" />:
        </strong>{" "}
        <Trans key="onboarding:_trn_search_syntax_text" />
      </li>
      <li>
        <strong>
          <Trans key="onboarding:_trn_search_syntax_urls_label" />:
        </strong>{" "}
        <Trans key="onboarding:_trn_search_syntax_urls" />
      </li>
      <li>
        <strong>
          <Trans key="onboarding:_trn_search_syntax_id_label" />:
        </strong>{" "}
        <Trans key="onboarding:_trn_search_syntax_ids" />
      </li>
      <li>
        <strong>
          <Trans key="onboarding:_trn_search_syntax_share_label" />:
        </strong>{" "}
        <Trans key="onboarding:_trn_search_syntax_share" />
      </li>
    </ul>
    <p class="mt-2">
      <Trans key="onboarding:_trn_search_syntax_addon_type" />
    </p>
    <p>
      <Trans key="onboarding:_trn_search_syntax_filters" />
    </p>
  </div>
)

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
    <OnboardingTip
      id="search-input-syntax"
      title={t("onboarding:_trn_search_syntax_title")}
      delay={200}
      description={<SearchSyntaxTips />}
      trigger="onClick"
      placement="bottom"
    >
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
          class="shrink-0 transition-[opacity,max-width] duration-300 ease-[cubic-bezier(.4,0,.2,1)]"
          classList={{
            "opacity-0 pointer-events-none max-w-0 overflow-hidden":
              !isExpanded(),
            "opacity-100 delay-[40ms]": isExpanded()
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
            class="placeholder:text-darkSlate-400 text-lightSlate-50 h-full min-w-0 flex-1 cursor-pointer bg-transparent text-sm outline-none"
            value=""
            readOnly
            onClick={handleSimpleClick}
          />
        </Show>

        <Show when={isExpanded()}>
          <input
            ref={expandedInputRef}
            placeholder={t("search:_trn_search_discover_anything")}
            class="placeholder:text-darkSlate-400 text-lightSlate-50 h-full min-w-0 flex-1 bg-transparent text-sm outline-none"
            value={searchResults?.searchQuery().searchQuery ?? ""}
            onInput={(e) => {
              searchResults?.setSearchQuery((prev) => ({
                ...prev,
                searchQuery: e.target.value
              }))
              if (isAddonPage()) {
                navigator.prev()
              }
            }}
          />
        </Show>

        <Show when={isExpanded() && searchResults?.isShareMode()}>
          <div class="bg-green-600/20 text-green-400 flex shrink-0 items-center gap-1 rounded px-2 py-0.5 text-xs">
            <div class="i-hugeicons:share-08 text-sm" />
            <span>{t("search:_trn_share")}</span>
          </div>
        </Show>
        <Show
          when={
            isExpanded() &&
            searchResults?.isDirectMode() &&
            !searchResults?.isShareMode()
          }
        >
          <div class="bg-primary-600/20 text-primary-400 flex shrink-0 items-center gap-1 rounded px-2 py-0.5 text-xs">
            <div class="i-hugeicons:link-01 text-sm" />
            <span>{t("search:_trn_direct")}</span>
          </div>
        </Show>

        <Show
          when={
            isExpanded() &&
            (searchResults?.searchQuery().searchQuery?.length || 0 > 0)
          }
        >
          <div
            class="i-hugeicons:cancel-01 text-lg text-darkSlate-500 cursor-pointer transition-colors duration-200 ease-spring hover:text-white"
            onClick={(e) => {
              e.preventDefault()
              e.stopPropagation()
              searchResults?.setSearchQuery((prev) => ({
                ...prev,
                searchQuery: ""
              }))
              if (isAddonPage()) {
                navigator.prev()
              }
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
          class="shrink-0 transition-[opacity,max-width] duration-300 ease-[cubic-bezier(.4,0,.2,1)]"
          classList={{
            "opacity-0 pointer-events-none max-w-0 overflow-hidden":
              !isExpanded(),
            "opacity-100 delay-[100ms]": isExpanded()
          }}
        >
          <Popover gutter={8} placement="bottom-end">
            <PopoverTrigger
              class="text-lightSlate-50 hover:bg-darkSlate-600 hover:text-white flex items-center justify-center rounded p-1.5 transition-colors duration-200"
              title={t("onboarding:_trn_search_syntax_title")}
            >
              <div class="i-hugeicons:help-circle text-lg" />
            </PopoverTrigger>
            <PopoverContent class="w-80" hideCloseButton>
              <div class="text-lightSlate-50 mb-2 font-semibold">
                {t("onboarding:_trn_search_syntax_title")}
              </div>
              <SearchSyntaxTips />
            </PopoverContent>
          </Popover>
        </div>

        <div
          class="shrink-0 transition-[opacity,max-width] duration-300 ease-[cubic-bezier(.4,0,.2,1)]"
          classList={{
            "opacity-0 pointer-events-none max-w-0 overflow-hidden":
              !isExpanded(),
            "opacity-100 delay-[110ms]": isExpanded()
          }}
        >
          <button
            class="flex items-center justify-center rounded p-1.5 transition-[colors,box-shadow] duration-200"
            classList={{
              "bg-darkSlate-600 text-white shadow-[inset_0_0_0_1px_rgb(var(--primary-500)/0.4)]":
                !!searchResults?.sidebarExpanded(),
              "text-lightSlate-50 hover:bg-darkSlate-600 hover:text-white":
                !searchResults?.sidebarExpanded()
            }}
            title={t("search:_trn_filters")}
            onClick={() => {
              searchResults?.setSidebarExpanded((prev) => !prev)
            }}
          >
            <div
              class="i-hugeicons:filter text-lg transition-transform duration-200 ease-[cubic-bezier(0.34,1.56,0.64,1)]"
              classList={{
                "-rotate-180": !!searchResults?.sidebarExpanded()
              }}
            />
          </button>
        </div>
      </div>
    </OnboardingTip>
  )
}
