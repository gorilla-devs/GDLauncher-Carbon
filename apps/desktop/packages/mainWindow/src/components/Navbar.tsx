import { useLocation } from "@solidjs/router"
import { Show } from "solid-js"
import { wideLogoUrl } from "@/utils/logos"
import { Tabs, TabsList, TabsTrigger, TabsIndicator } from "@gd/ui"
import { useGDNavigate } from "@/managers/NavigationManager"
import { AccountsDropdown } from "./AccountsDropdown"

import { useGlobalStore } from "./GlobalStoreContext"
import { EnhancedSearchBar } from "./EnhancedSearchBar"
import { hasPendingUpdate, showPendingUpdateToast } from "@/utils/updater"
import { useTransContext } from "@gd/i18n"
import { DecorationSlot } from "@/themes/DecorationSlot"
import { NAV_BAND_CLIP } from "@/themes/decorations"
import { layout, NAV_METRICS } from "@/themes/layout"
import { useNavState } from "./navState"

// The top bar of the standard and compact layouts; the rail is NavRail.
const AppNavbar = () => {
  const location = useLocation()
  const navigator = useGDNavigate()
  const globalStore = useGlobalStore()

  const [t] = useTransContext()

  const { isLogin, selectedValue, accounts } = useNavState()
  const metrics = () =>
    layout().nav === "compact" ? NAV_METRICS.compact : NAV_METRICS.standard

  return (
    <Show when={!isLogin() && layout().nav !== "rail"}>
      <nav
        data-gdl="nav"
        class="disable-view-transition relative bg-darkSlate-800 text-lightSlate-50 flex items-center justify-between gap-8 px-5"
        style={{
          height: `${metrics().height}px`,
          // Popovers and tooltips portal out of the nav, so clipping is safe.
          ...NAV_BAND_CLIP
        }}
      >
        <div
          data-gdl="nav.home"
          class="group relative z-0 flex h-full w-fit items-center"
          onClick={() => navigator.navigate("/library")}
        >
          <div
            data-gdl="nav.home.highlight"
            class="bg-darkSlate-700 -z-1 absolute left-0 top-0 h-full w-full scale-75 rounded-md opacity-0 transition-[transform,opacity] duration-150 ease-[cubic-bezier(.4,0,.2,1)] group-hover:scale-100 group-hover:opacity-100"
            style={{
              "will-change": "transform, opacity"
            }}
          />
          <div class="flex items-center gap-2 px-2">
            <div
              data-gdl="nav.back"
              class="i-hugeicons:arrow-left-01 h-6 w-6 transition-[transform,opacity] duration-200 ease-[cubic-bezier(.4,0,.2,1)]"
              classList={{
                "opacity-0 -translate-x-4": location.pathname === "/library"
              }}
              style={{
                "will-change": "transform, opacity"
              }}
            />
            <div
              data-gdl="nav.logo"
              class="relative isolate flex transition-transform duration-200 ease-[cubic-bezier(.4,0,.2,1)]"
              classList={{
                "-translate-x-4": location.pathname === "/library"
              }}
              style={{
                "will-change": "transform"
              }}
            >
              <img
                data-testid="navbar-logo"
                src={wideLogoUrl}
                class="max-w-none"
                style={{ height: `${metrics().logo}px` }}
              />
              <DecorationSlot anchor="nav.logo" />
            </div>
          </div>
        </div>
        <div
          data-gdl="nav.search"
          class="flex w-full items-center justify-center gap-4"
        >
          <EnhancedSearchBar size={metrics().search} />
        </div>
        <div
          data-gdl="nav.actions"
          class="text-lightSlate-50 flex h-full list-none items-center gap-6"
        >
          <Show when={hasPendingUpdate()}>
            <div
              data-gdl="nav.update"
              class="animate-icons-on-hover cursor-pointer text-green-500 hover:text-green-400 transition-colors"
              onClick={showPendingUpdateToast}
              title={t("app:_trn_update_pending_tooltip")}
            >
              <div class="i-hugeicons:download-04 text-2xl" />
            </div>
          </Show>
          <Tabs
            value={selectedValue()}
            class="h-auto w-auto flex-row items-center"
          >
            <TabsList class="bg-transparent gap-4 h-auto">
              <Show when={selectedValue()}>
                <TabsIndicator />
              </Show>
              <TabsTrigger
                value="settings"
                class="p-2"
                data-testid="navbar-settings"
                data-gdl="nav.settings"
                onClick={() => navigator.navigate("/settings")}
              >
                <div
                  data-gdl="tab.icon"
                  class="i-hugeicons:settings-01 h-6 w-6"
                />
              </TabsTrigger>
              <TabsTrigger
                value="news"
                class="p-2"
                data-gdl="nav.news"
                onClick={() => navigator.navigate("/news")}
              >
                <div data-gdl="tab.icon" class="i-hugeicons:news h-6 w-6" />
              </TabsTrigger>
            </TabsList>
          </Tabs>
          <div
            data-gdl="account-menu"
            class="mr-6 flex justify-end lg:min-w-fit"
          >
            <Show when={globalStore.accounts.data}>
              <AccountsDropdown
                dense={layout().nav === "compact"}
                accounts={accounts()}
                value={globalStore.currentlySelectedAccountUuid.data}
              />
            </Show>
          </div>
        </div>
        {/* The nav is not isolated, so the slot's z-index would leak into the root
            stacking context and paint over the content column's effects. */}
        <DecorationSlot anchor="nav" class="!z-0" />
      </nav>
    </Show>
  )
}

export default AppNavbar
