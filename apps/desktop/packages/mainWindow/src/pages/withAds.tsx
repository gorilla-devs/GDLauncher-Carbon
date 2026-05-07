import { AdsBanner } from "@/components/AdBanner"
import { TopBannerAd } from "@/components/TopBannerAd"
import AppNavbar from "@/components/Navbar"
import { createSignal, JSX, Show } from "solid-js"

import adSize, { bannerAdSize, hideAdText } from "@/utils/adhelper"
import { Trans } from "@gd/i18n"
import { Button } from "@gd/ui"
import { useModal } from "@/managers/ModalsManager"
import { SearchInputContext } from "@/components/SearchInputContext"
import { getSearchResults } from "@/utils/platformSearch"
import ThemedPatternSVG from "@/components/ThemedPatternSVG"
import { rspc } from "@/utils/rspcClient"
import { formatBytes } from "@/utils/formatBytes"

// Threshold for showing the cache-bloat banner. Triggered off the
// `gdlauncher` portion of cache (DB + temp + logs) — that's where the
// HTTPCache bloat bug lives. We deliberately don't trigger on the
// `minecraft` portion because assets/libraries/natives grow legitimately
// with installed instances. 2 GB is well above a healthy GDLauncher cache
// (typically <100 MB) and small enough to surface the problem early.
const GDL_BLOAT_THRESHOLD_BYTES = 2 * 1024 * 1024 * 1024

// Session-scoped dismiss signal — returns on relaunch until the user actually
// cleans up (at which point the gdlauncher size drops below threshold).
const [cacheBannerDismissed, setCacheBannerDismissed] = createSignal(false)

function withAdsLayout(props: { children?: JSX.Element }) {
  const modalContext = useModal()
  const searchResults = getSearchResults({
    limit: 20,
    offset: 0
  })

  // Per-scope cache sizes. Same query key as Settings + the cleanup modal,
  // so the directory walk runs once per cache window and feeds all three.
  const cacheSizes = rspc.createQuery(() => ({
    queryKey: ["settings.getCacheSizes"]
  }))

  // Banner total matches Settings: gdlauncher + minecraft.
  const totalCacheSize = () =>
    cacheSizes.data
      ? cacheSizes.data.gdlauncher + cacheSizes.data.minecraft
      : undefined

  const showCacheBanner = () =>
    !cacheBannerDismissed() &&
    cacheSizes.data !== undefined &&
    cacheSizes.data.gdlauncher > GDL_BLOAT_THRESHOLD_BYTES

  return (
    <SearchInputContext.Provider value={searchResults}>
      <div class="flex h-screen flex-col">
        <AppNavbar />
        <Show when={showCacheBanner()}>
          <div class="bg-darkSlate-800 border-b border-darkSlate-700 shrink-0 px-6 py-3">
            <div class="bg-darkSlate-700/50 border-yellow-500/30 flex items-center gap-4 rounded-lg border-l-4 px-4 py-3">
              <div class="bg-yellow-500/15 text-yellow-400 flex h-9 w-9 shrink-0 items-center justify-center rounded-full">
                <div class="i-hugeicons:alert-02 h-5 w-5" />
              </div>
              <div class="flex min-w-0 flex-1 flex-col">
                <div class="text-lightSlate-50 text-sm font-semibold">
                  <Trans
                    key="settings:_trn_cache_storage_bloat_banner_title"
                    options={{ size: formatBytes(totalCacheSize()!) }}
                  />
                </div>
                <div class="text-lightSlate-500 text-xs">
                  <Trans key="settings:_trn_cache_storage_bloat_banner_desc" />
                </div>
              </div>
              <Button
                type="primary"
                size="small"
                onClick={() => {
                  modalContext?.openModal({ name: "cacheCleanup" })
                }}
              >
                <div class="i-hugeicons:delete-02 h-4 w-4" />
                <Trans key="settings:_trn_cache_storage_clean_up" />
              </Button>
              <button
                class="i-hugeicons:cancel-01 text-lightSlate-500 hover:text-lightSlate-50 h-4 w-4 shrink-0 cursor-pointer transition-colors"
                onClick={() => setCacheBannerDismissed(true)}
                aria-label="Dismiss"
              />
            </div>
          </div>
        </Show>
        <div class="z-99 flex min-h-0 w-screen flex-1">
          <main class="relative grow">
            <div class="flex h-full justify-end">
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
      </div>
    </SearchInputContext.Provider>
  )
}

export default withAdsLayout
