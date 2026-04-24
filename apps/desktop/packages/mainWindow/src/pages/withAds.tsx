import { AdsBanner } from "@/components/AdBanner"
import { TopBannerAd } from "@/components/TopBannerAd"
import AppNavbar from "@/components/Navbar"
import { createSignal, JSX, Show } from "solid-js"

import adSize, { bannerAdSize, hideAdText } from "@/utils/adhelper"
import { Trans } from "@gd/i18n"
import { useModal } from "@/managers/ModalsManager"
import { SearchInputContext } from "@/components/SearchInputContext"
import { getSearchResults } from "@/utils/platformSearch"
import ThemedPatternSVG from "@/components/ThemedPatternSVG"
import { rspc } from "@/utils/rspcClient"
import { formatBytes } from "@/utils/formatBytes"

// Threshold for showing the DB-bloat banner. This tracks only the SQLite
// database size, not total cache footprint — the banner's purpose is to flag
// the HTTPCache bloat bug (which lives in the DB), not normal disk usage
// like `assets/` and `libraries/` which grow legitimately with installed
// instances. 2 GB is well above a healthy DB (typically <100 MB) and small
// enough to surface the problem before it's catastrophic.
const DB_BLOAT_THRESHOLD_BYTES = 2 * 1024 * 1024 * 1024

// Session-scoped dismiss signal — returns on relaunch until the user actually
// cleans up (at which point getDbSize naturally drops below threshold).
const [cacheBannerDismissed, setCacheBannerDismissed] = createSignal(false)

function withAdsLayout(props: { children?: JSX.Element }) {
  const modalContext = useModal()
  const searchResults = getSearchResults({
    limit: 20,
    offset: 0
  })

  // Cheap DB-size poll (two file stats). Banner only cares about DB bloat
  // specifically, so we don't pay for the disk walks that
  // `getTotalCacheSize` would trigger.
  const dbSize = rspc.createQuery(() => ({
    queryKey: ["settings.getDbSize"]
  }))

  const showCacheBanner = () =>
    !cacheBannerDismissed() &&
    dbSize.data !== undefined &&
    dbSize.data > DB_BLOAT_THRESHOLD_BYTES

  return (
    <SearchInputContext.Provider value={searchResults}>
      <AppNavbar />
      <Show when={showCacheBanner()}>
        <div class="bg-yellow-900/40 border-yellow-700 text-yellow-100 flex items-center justify-between gap-4 border-b px-6 py-2 text-sm">
          <div class="flex items-center gap-2">
            <div class="i-hugeicons:alert-02 h-4 w-4 shrink-0" />
            <Trans
              key="settings:_trn_cache_storage_bloat_banner"
              options={{ size: formatBytes(dbSize.data!) }}
            />
          </div>
          <div class="flex items-center gap-2">
            <button
              class="bg-yellow-700 hover:bg-yellow-600 rounded px-3 py-1 text-xs font-medium transition-colors"
              onClick={() => {
                modalContext?.openModal({ name: "cacheCleanup" })
              }}
            >
              <Trans key="settings:_trn_cache_storage_clean_up" />
            </button>
            <button
              class="i-hugeicons:cancel-01 hover:text-yellow-50 h-4 w-4 cursor-pointer opacity-70 transition-opacity"
              onClick={() => setCacheBannerDismissed(true)}
              aria-label="Dismiss"
            />
          </div>
        </div>
      </Show>
      <div class="z-99 flex h-auto w-screen">
        <main class="relative grow">
          <div class="flex h-[calc(100vh-60px)] justify-end">
            <div
              style={{
                width: `calc(100vw - ${adSize.width}px)`
              }}
            >
              {props.children}
            </div>
            <Show when={adSize.shouldShow}>
              <div
                class="relative flex h-full flex-col gap-2 items-center"
                style={{
                  width: `${adSize.width}px`,
                  "view-transition-name": `ad`,
                  "z-index": "50000"
                }}
              >
                <div class="absolute inset-0 overflow-hidden pointer-events-none">
                  <ThemedPatternSVG />
                </div>
                <Show when={bannerAdSize.shouldShow}>
                  <div
                    class="relative z-10"
                    style={{
                      width: `${bannerAdSize.width}px`,
                      height: `${bannerAdSize.height}px`
                    }}
                  >
                    <TopBannerAd />
                  </div>
                </Show>
                <div
                  class="relative z-10"
                  style={{
                    width: `${adSize.width}px`,
                    height: `${adSize.height}px`
                  }}
                >
                  <AdsBanner />
                </div>
                <Show when={!hideAdText()}>
                  <div class="relative z-10 flex justify-center">
                    <div
                      class="hover:text-lightSlate-50 text-lightSlate-700 text-center transition-colors duration-200"
                      onClick={() => {
                        modalContext?.openModal({
                          name: "whyAreAdsNeeded"
                        })
                      }}
                    >
                      <Trans key="ads:_trn_why_are_ads_needed" />
                    </div>
                  </div>
                </Show>
              </div>
            </Show>
          </div>
        </main>
      </div>
    </SearchInputContext.Provider>
  )
}

export default withAdsLayout
