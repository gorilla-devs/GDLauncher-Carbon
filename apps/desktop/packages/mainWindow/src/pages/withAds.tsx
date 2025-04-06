import { AdsBanner } from "@/components/AdBanner"
import AppNavbar from "@/components/Navbar"
import { Outlet } from "@solidjs/router"
import { Match, Show, Switch } from "solid-js"

import adSize from "@/utils/adhelper"
import { Trans } from "@gd/i18n"
import { useModal } from "@/managers/ModalsManager"
import { BisectBanner } from "@/components/BisectBanner"

function withAdsLayout() {
  const modalContext = useModal()

  return (
    <>
      <AppNavbar />
      <div
        class="z-99 flex h-auto w-screen"
        style={{
          background: "var(--ads-sidebar-background)"
        }}
      >
        <main class="relative flex-grow">
          <div class="flex h-[calc(100vh-60px)] justify-end">
            <div
              style={{
                width: `calc(100vw - ${adSize.width}px)`
              }}
            >
              <Outlet />
            </div>
            <div class="flex h-full flex-col justify-between gap-4">
              <div
                // class="py-4"
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
    </>
  )
}

export default withAdsLayout
