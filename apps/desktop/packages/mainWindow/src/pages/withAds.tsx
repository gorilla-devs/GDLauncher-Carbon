import AppNavbar from "@/components/Navbar"
import NavRail from "@/components/NavRail"
import { EnhancedSearchBar } from "@/components/EnhancedSearchBar"
import { useLocation } from "@solidjs/router"
import {
  createEffect,
  createSignal,
  JSX,
  onCleanup,
  onMount,
  Show
} from "solid-js"

import adSize, { bannerAdSize, setAdLayoutMounted } from "@/utils/adhelper"
import { Trans, useTransContext } from "@gd/i18n"
import { Button } from "@gd/ui"
import { useModal } from "@/managers/ModalsManager"
import { PAGE_ID } from "@/managers/NavigationManager"
import { SearchInputContext } from "@/components/SearchInputContext"
import { getSearchResults } from "@/utils/platformSearch"
import { activeTheme } from "@/themes/active"
import { DecorationSlot } from "@/themes/DecorationSlot"
import { CONTENT_COLUMN_CLIP, NAV_BAND_CLIP } from "@/themes/decorations"
import { EffectsLayer } from "@/themes/effects/EffectsLayer"
import {
  adColumnSpacer,
  layout,
  railWidth,
  showsSearchStrip
} from "@/themes/layout"
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
  const [t] = useTransContext()
  const location = useLocation()
  onMount(() => setAdLayoutMounted(true))
  onCleanup(() => setAdLayoutMounted(false))

  // The theme's nav: a top bar (standard, compact) or a rail at the left.
  // Main's ad view never moves; every layout reserves its column at full
  // height and only lines the column's art up with it.
  const nav = () => layout().nav
  const rail = () => nav() === "rail"
  // On a display too short for the ad under the top bar, the ad column runs
  // beside it, the whole height of the window: the bar and the banner end
  // where the column starts, and main puts the ad view at the window's top.
  const beside = () => !!adSize.besideNavbar
  const besideTopBar = () =>
    beside() && !rail() ? `calc(100vw - ${adSize.width}px)` : undefined
  const spacer = () => (beside() ? 0 : adColumnSpacer(nav()))

  // The ad itself lives in a separate view owned by the main process; the page
  // only provides its label and handles its click.
  createEffect(() => {
    window.adView.setLabels(t("ads:_trn_why_are_ads_needed"))
  })

  // Lets popovers/themes keep out of the column the main process positions the
  // ad view in. Only while this layout (and so the column) is on screen.
  createEffect(() => {
    document.documentElement.style.setProperty(
      "--ad-column-width",
      adSize.shouldShow ? `${adSize.width}px` : "0px"
    )
  })
  onCleanup(() => {
    document.documentElement.style.setProperty("--ad-column-width", "0px")
  })
  // Lets fixed-position bars start right of the rail.
  createEffect(() => {
    document.documentElement.style.setProperty(
      "--gdl-rail-width",
      `${railWidth(nav())}px`
    )
  })
  onCleanup(() => {
    document.documentElement.style.setProperty("--gdl-rail-width", "0px")
  })

  onMount(() => {
    const unsubscribe = window.adView.onOpenWhyAds(() => {
      modalContext?.openModal({ name: "whyAreAdsNeeded" })
    })
    onCleanup(unsubscribe)
  })
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
      <div class="flex h-screen" classList={{ "flex-col": !rail() }}>
        <div
          class="shrink-0"
          data-testid="ads-layout-top"
          style={{ width: besideTopBar() }}
        >
          <AppNavbar />
        </div>
        <Show when={rail()}>
          <NavRail />
        </Show>
        <div class="flex min-h-0 min-w-0 flex-1 flex-col">
          <Show when={showCacheBanner()}>
            <div
              class="bg-darkSlate-800 border-b border-darkSlate-700 shrink-0 px-6 py-3"
              style={{ width: besideTopBar() }}
            >
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
          <div
            class="flex min-h-0 flex-1"
            classList={{ "w-screen": !rail(), "w-full": rail() }}
          >
            <main class="relative grow">
              <div class="flex h-full justify-end">
                {/* The search strip, when there is one, sits above the content
                  column, so the column's effects and decorations start under
                  it, as they start under a top bar. */}
                <div
                  class="flex h-full flex-col"
                  style={{
                    width: `calc(100vw - ${railWidth(nav())}px - ${adSize.width}px)`
                  }}
                >
                  <Show when={showsSearchStrip(nav(), location.pathname)}>
                    <div
                      data-gdl="nav nav.search"
                      class="bg-darkSlate-800 flex h-[60px] shrink-0 items-center px-5"
                      style={NAV_BAND_CLIP}
                    >
                      <EnhancedSearchBar />
                    </div>
                  </Show>
                  <div
                    data-gdl="content"
                    class="relative isolate min-h-0 flex-1"
                    style={CONTENT_COLUMN_CLIP}
                  >
                    <div id={PAGE_ID} class="relative z-0 h-full">
                      {props.children}
                    </div>
                    <DecorationSlot anchor="content" />
                    <EffectsLayer />
                  </div>
                </div>
                <Show when={adSize.shouldShow}>
                  {/* Keeps the column's place in the row while the column
                      itself is taken out of it below. */}
                  <Show when={beside()}>
                    <div
                      class="shrink-0"
                      style={{ width: `${adSize.width}px` }}
                    />
                  </Show>
                  <div
                    class="flex flex-col gap-2 items-center"
                    classList={{
                      "relative h-full": !beside(),
                      "fixed right-0 top-0 h-screen": beside(),
                      // The backdrop colour is the column's own, so a theme's
                      // ad-column-background rule replaces it; the logo
                      // pattern draws on top in its own layer.
                      "bg-darkSlate-900": !activeTheme()?.background
                    }}
                    data-gdl="ad-column-background"
                    data-testid="ads-column"
                    style={{
                      width: `${adSize.width}px`,
                      "view-transition-name": `ad`,
                      // Lines the placeholders up with the ad view, which stays
                      // at main's top under every nav. Padding, not a child: the
                      // column's gap would add to a spacer element.
                      "padding-top": spacer() ? `${spacer()}px` : undefined
                    }}
                  >
                    <Show when={!activeTheme()?.background}>
                      <div
                        data-gdl="ad-column-background.pattern"
                        class="text-darkSlate-800 absolute inset-0 overflow-hidden pointer-events-none"
                      >
                        <ThemedPatternSVG />
                      </div>
                    </Show>
                    <Show when={bannerAdSize.shouldShow}>
                      <div
                        class="relative z-10"
                        data-testid="ad-slot"
                        style={{
                          width: `${bannerAdSize.width}px`,
                          height: `${bannerAdSize.height}px`
                        }}
                      />
                    </Show>
                    <div
                      class="relative z-10"
                      data-testid="ad-slot"
                      style={{
                        width: `${adSize.width}px`,
                        height: `${adSize.height}px`
                      }}
                    />
                  </div>
                </Show>
              </div>
            </main>
          </div>
        </div>
      </div>
    </SearchInputContext.Provider>
  )
}

export default withAdsLayout
