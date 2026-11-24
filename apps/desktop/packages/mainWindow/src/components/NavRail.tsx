import { Show } from "solid-js"
import {
  Tabs,
  TabsIndicator,
  TabsList,
  TabsTrigger,
  Tooltip,
  TooltipContent,
  TooltipTrigger
} from "@gd/ui"
import { useTransContext } from "@gd/i18n"
import { useGDNavigate } from "@/managers/NavigationManager"
import { logoUrl } from "@/utils/logos"
import { hasPendingUpdate, showPendingUpdateToast } from "@/utils/updater"
import { DecorationSlot } from "@/themes/DecorationSlot"
import { NAV_BAND_CLIP } from "@/themes/decorations"
import { AccountsDropdown } from "./AccountsDropdown"
import { useGlobalStore } from "./GlobalStoreContext"
import { useNavState } from "./navState"

const ICON_BUTTON =
  "flex h-10 w-10 cursor-pointer items-center justify-center rounded-lg border-0 bg-transparent p-0 transition-colors duration-100 ease-spring hover:bg-darkSlate-700"

// The rail layout's nav: a 64 px column of icons along the window's left edge.
// It has no room for the search box, so the search pages show it in a strip
// above the page instead (withAds.tsx). Like the top bar, it clips to its own
// band, and its tooltips and menu open to the right, away from the ad column.
const NavRail = () => {
  const navigator = useGDNavigate()
  const globalStore = useGlobalStore()
  const [t] = useTransContext()
  const { isLogin, selectedValue, accounts } = useNavState()

  return (
    <Show when={!isLogin()}>
      <nav
        data-gdl="nav nav.rail"
        class="disable-view-transition relative bg-darkSlate-800 text-lightSlate-50 flex h-full w-16 shrink-0 flex-col items-center gap-2 py-3"
        style={NAV_BAND_CLIP}
      >
        <Tooltip placement="right">
          <TooltipTrigger
            as="button"
            type="button"
            aria-label={t("library:_trn_library")}
            class={ICON_BUTTON}
            onClick={() => navigator.navigate("/library")}
          >
            <div data-gdl="nav.logo" class="relative isolate flex">
              <img data-testid="navbar-logo" src={logoUrl} class="h-8 w-8" />
              <DecorationSlot anchor="nav.logo" />
            </div>
          </TooltipTrigger>
          <TooltipContent>{t("library:_trn_library")}</TooltipContent>
        </Tooltip>
        <Tooltip placement="right">
          <TooltipTrigger
            as="button"
            type="button"
            data-gdl="nav.search"
            aria-label={t("search:_trn_search")}
            class={`${ICON_BUTTON} text-lightSlate-600 hover:text-lightSlate-50`}
            onClick={() => navigator.navigate("/search")}
          >
            <div class="i-hugeicons:search-01 h-6 w-6" />
          </TooltipTrigger>
          <TooltipContent>{t("search:_trn_search")}</TooltipContent>
        </Tooltip>
        <Show when={hasPendingUpdate()}>
          <Tooltip placement="right">
            <TooltipTrigger
              as="button"
              type="button"
              data-gdl="nav.update"
              aria-label={t("app:_trn_update_pending_tooltip")}
              class={`${ICON_BUTTON} animate-icons-on-hover text-green-500 hover:text-green-400`}
              onClick={showPendingUpdateToast}
            >
              <div class="i-hugeicons:download-04 h-6 w-6" />
            </TooltipTrigger>
            <TooltipContent>
              {t("app:_trn_update_pending_tooltip")}
            </TooltipContent>
          </Tooltip>
        </Show>
        <div class="flex-1" />
        <Tabs
          value={selectedValue()}
          orientation="vertical"
          class="h-auto w-auto flex-col items-center"
        >
          <TabsList class="bg-transparent h-auto flex-col gap-2">
            <Show when={selectedValue()}>
              <TabsIndicator />
            </Show>
            {/* The tab is the tooltip's trigger, so keyboard focus on it
                opens the tooltip too. */}
            <Tooltip placement="right">
              <TooltipTrigger
                as={TabsTrigger}
                value="settings"
                class="p-2"
                data-testid="navbar-settings"
                data-gdl="nav.settings"
                aria-label={t("ui:_trn_settings")}
                onClick={() => navigator.navigate("/settings")}
              >
                <div
                  data-gdl="tab.icon"
                  class="i-hugeicons:settings-01 h-6 w-6"
                />
              </TooltipTrigger>
              <TooltipContent>{t("ui:_trn_settings")}</TooltipContent>
            </Tooltip>
            <Tooltip placement="right">
              <TooltipTrigger
                as={TabsTrigger}
                value="news"
                class="p-2"
                data-gdl="nav.news"
                aria-label={t("news:_trn_news")}
                onClick={() => navigator.navigate("/news")}
              >
                <div data-gdl="tab.icon" class="i-hugeicons:news h-6 w-6" />
              </TooltipTrigger>
              <TooltipContent>{t("news:_trn_news")}</TooltipContent>
            </Tooltip>
          </TabsList>
        </Tabs>
        <div data-gdl="account-menu" class="flex justify-center">
          <Show when={globalStore.accounts.data}>
            <AccountsDropdown
              compact
              accounts={accounts()}
              value={globalStore.currentlySelectedAccountUuid.data}
            />
          </Show>
        </div>
        {/* Not isolated, like the top bar: the slot stays in the nav's own
            stacking order, under the content column's effects. */}
        <DecorationSlot anchor="nav" class="!z-0" />
      </nav>
    </Show>
  )
}

export default NavRail
