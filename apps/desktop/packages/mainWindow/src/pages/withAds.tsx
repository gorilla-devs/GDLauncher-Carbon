import { AdsBanner } from "@/components/AdBanner"
import AppNavbar from "@/components/Navbar"
import { Outlet } from "@solidjs/router"
import { Match, Show, Switch } from "solid-js"

import adSize from "@/utils/adhelper"
import { Trans } from "@gd/i18n"
import { useModal } from "@/managers/ModalsManager"
import { BisectBanner } from "@/components/BisectBanner"
import { SearchInputContext } from "@/components/SearchInputContext"
import { getSearchResults } from "@/utils/platformSearch"

function withAdsLayout() {
  const modalContext = useModal()
  const searchResults = getSearchResults({
    limit: 20,
    offset: 0
  })

  return (
    <SearchInputContext.Provider value={searchResults}>
      <AppNavbar />
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
            <div
              class="flex h-full flex-col justify-between gap-4"
              style={{
                "view-transition-name": `ad`,
                background: "var(--ads-sidebar-background)"
              }}
            >
              <div
                style={{
                  width: `${adSize.width}px`,
                  height: `${adSize.height}px`
                }}
              >
                <Show when={adSize.shouldShow}>
                  <Switch>
                    <Match when={adSize.useFallbackAd}>
                      <BisectBanner />
                    </Match>
                    <Match when={!adSize.useFallbackAd}>
                      <AdsBanner />
                    </Match>
                  </Switch>
                </Show>
              </div>
              <div class="flex justify-center">
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
          </div>
        </main>
      </div>
    </SearchInputContext.Provider>
  )
}

export default withAdsLayout
