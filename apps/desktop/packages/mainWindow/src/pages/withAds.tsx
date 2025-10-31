import { AdsBanner } from "@/components/AdBanner"
import AppNavbar from "@/components/Navbar"
import { Outlet } from "@solidjs/router"
import { Show } from "solid-js"

import adSize from "@/utils/adhelper"
import { Trans } from "@gd/i18n"
import { useModal } from "@/managers/ModalsManager"
import { SearchInputContext } from "@/components/SearchInputContext"
import { getSearchResults } from "@/utils/platformSearch"
import { FilterBadgesBar } from "@/components/FilterBadgesBar"
import ThemedPatternSVG from "@/components/ThemedPatternSVG"

function withAdsLayout() {
  const modalContext = useModal()
  const searchResults = getSearchResults({
    limit: 20,
    offset: 0
  })

  return (
    <SearchInputContext.Provider value={searchResults}>
      <AppNavbar />
      <FilterBadgesBar />
      <div class="z-99 flex h-auto w-screen">
        <main class="relative grow">
          <div class="flex h-[calc(100vh-60px)] justify-end">
            <div
              style={{
                width: `calc(100vw - ${adSize.width}px)`
              }}
            >
              <Outlet />
            </div>
            <Show when={adSize.shouldShow}>
              <div
                class="relative flex h-full flex-col justify-between gap-4"
                style={{ width: `${adSize.width}px` }}
              >
                <div class="absolute inset-0 overflow-hidden pointer-events-none">
                  <ThemedPatternSVG />
                </div>
                <div
                  class="relative z-10"
                  style={{
                    "view-transition-name": `ad`,
                    width: `${adSize.width}px`,
                    height: `${adSize.height}px`
                  }}
                >
                  <AdsBanner />
                </div>
                <div class="relative z-10 flex justify-center">
                  <div
                    class="hover:text-lightSlate-50 text-lightSlate-700 text-center transition-colors duration-200"
                    onClick={() => {
                      modalContext?.openModal({
                        name: "whyAreAdsNeeded"
                      })
                    }}
                  >
                    <Trans key="why_are_ads_needed" />
                  </div>
                </div>
              </div>
            </Show>
          </div>
        </main>
      </div>
    </SearchInputContext.Provider>
  )
}

export default withAdsLayout
