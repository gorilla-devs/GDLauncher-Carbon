import { useLocation, useMatch } from "@solidjs/router"
import { Match, Show, Switch, createMemo } from "solid-js"
import GDLauncherWideLogo from "/assets/images/gdlauncher_wide_logo_blue.svg"
import {
  Tab,
  TabList,
  Tabs,
  Tooltip,
  Button,
  Input,
  TooltipContent,
  TooltipTrigger
} from "@gd/ui"
import { useGDNavigate } from "@/managers/NavigationManager"
import { AccountsDropdown } from "./AccountsDropdown"
import { AccountStatus, AccountType } from "@gd/core_module/bindings"
import { createStore } from "solid-js/store"
import { port } from "@/utils/rspcClient"
import updateAvailable, {
  updateDownloaded,
  updateProgress
} from "@/utils/updater"
import { Trans, useTransContext } from "@gd/i18n"
import { useModal } from "@/managers/ModalsManager"
import { useGlobalStore } from "./GlobalStoreContext"
import useSearchContext from "./SearchInputContext"

export interface AccountsStatus {
  label: {
    name: string
    icon: string | undefined
    uuid: string
    type: AccountType
    status: AccountStatus | undefined
  }
  key: string
}

const AppNavbar = () => {
  const location = useLocation()
  const navigator = useGDNavigate()
  const globalStore = useGlobalStore()
  const modalsContext = useModal()
  const [t] = useTransContext()

  const searchResults = useSearchContext()

  const isLogin = useMatch(() => "/")
  const isSettings = useMatch(() => "/settings")
  const isSettingsNested = useMatch(() => "/settings/*")
  const isNews = useMatch(() => "/news/*")
  const isSearch = useMatch(() => "/search/*")
  const selectedIndex = () => {
    if (isSettings() || isSettingsNested()) return 0
    if (isNews()) return 1
    return -1
  }

  const accounts = createMemo(() => {
    return (
      globalStore.accounts.data?.map((account) => {
        const accountStatusQuery = {} as any

        return {
          label: {
            name: account?.username,
            icon: `http://127.0.0.1:${port}/account/headImage?uuid=${account.uuid}`,
            uuid: account.uuid,
            type: account.type,
            status: accountStatusQuery.data
          },
          key: account?.uuid
        }
      }) ?? []
    )
  })

  return (
    <Show when={!isLogin()}>
      <nav
        class="disable-view-transition bg-darkSlate-800 text-lightSlate-50 flex items-center justify-between gap-8 px-5"
        style={{
          height: "60px"
        }}
      >
        <div
          class="group relative z-0 flex h-full w-fit items-center"
          onClick={() => navigator.navigate("/library")}
        >
          <div class="bg-darkSlate-700 -z-1 absolute left-0 top-0 h-full w-full scale-75 rounded-md opacity-0 transition-[transform,opacity] duration-150 ease-[cubic-bezier(.4,0,.2,1)] group-hover:scale-100 group-hover:opacity-100" />
          <div class="flex items-center gap-2 px-2">
            <div
              class="i-ri:arrow-left-s-fill h-6 w-6 transition-[transform,opacity] duration-200 ease-[cubic-bezier(.4,0,.2,1)]"
              classList={{
                "opacity-0 -translate-x-4": location.pathname === "/library"
              }}
            />
            <img
              src={GDLauncherWideLogo}
              class="h-9 max-w-none transition-transform duration-200 ease-[cubic-bezier(.4,0,.2,1)]"
              classList={{
                "-translate-x-4": location.pathname === "/library"
              }}
            />
          </div>
        </div>
        <div class="flex w-full items-center justify-center gap-4">
          <Input
            placeholder={t("search.search_discover_anything")}
            containerClass="px-10"
            class="w-80"
            tabIndex={0}
            value={searchResults?.searchQuery().searchQuery ?? ""}
            onFocus={() => {
              if (isSearch()) {
                return
              }

              navigator.navigate("/search")
            }}
            onInput={(e) => {
              searchResults?.setSearchQuery((prev) => ({
                ...prev,
                searchQuery: e.target.value
              }))
            }}
            icon={
              <div class="flex items-center gap-1">
                <Show
                  when={
                    searchResults?.searchQuery().searchQuery?.length || 0 > 0
                  }
                >
                  <div
                    class="i-ri:close-line text-darkSlate-500 text-xl transition-colors duration-200 ease-in-out hover:text-white"
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
              </div>
            }
          />
          <Button
            class="w-max"
            size="small"
            type="primary"
            onClick={() => {
              modalsContext?.openModal({
                name: "instanceCreation"
              })
            }}
          >
            <i class="i-ri:add-fill flex" />
          </Button>
        </div>
        <div class="text-lightSlate-50 flex h-full list-none items-center gap-6">
          <Tabs index={selectedIndex()}>
            <TabList aligment="between">
              <div class="flex items-center gap-6">
                <div
                  onClick={() => {
                    if (!(!!isSettings() || !!isSettingsNested()))
                      navigator.navigate("/settings")
                  }}
                >
                  <Tab>
                    <div
                      class="i-ri:settings-3-fill text-2xl"
                      classList={{
                        "text-lightSlate-50":
                          !!isSettings() || !!isSettingsNested()
                      }}
                    />
                  </Tab>
                </div>
                <div
                  onClick={() => {
                    navigator.navigate("/news")
                  }}
                >
                  <Tab>
                    <div
                      class="i-ri:news-fill text-2xl"
                      classList={{
                        "text-lightSlate-50": !!isNews()
                      }}
                    />
                  </Tab>
                </div>
                <Show
                  when={
                    updateAvailable() ||
                    updateDownloaded() ||
                    updateProgress() > 0
                  }
                >
                  <Tab ignored>
                    <Tooltip>
                      <TooltipTrigger>
                        <div
                          class="i-ri:download-2-fill text-2xl text-green-500"
                          classList={{
                            "hover:text-green-100": !updateDownloaded()
                          }}
                          onClick={() => {
                            if (updateDownloaded()) {
                              window.installUpdate()
                            } else {
                              modalsContext?.openModal({ name: "appUpdate" })
                            }
                          }}
                        />
                      </TooltipTrigger>
                      <TooltipContent>
                        <Switch>
                          <Match when={updateDownloaded()}>
                            <Trans key="app_update.apply_and_restart" />
                          </Match>
                          <Match when={updateProgress() > 0}>
                            <Trans
                              key="app_update.downloading"
                              options={{
                                progress: updateProgress()
                              }}
                            />
                          </Match>
                          <Match when={updateAvailable()}>
                            <Trans key="app_update.new_update_available_text" />
                          </Match>
                        </Switch>
                      </TooltipContent>
                    </Tooltip>
                  </Tab>
                </Show>
              </div>
            </TabList>
          </Tabs>
          <div class="mr-6 flex justify-end lg:min-w-fit">
            <Show when={globalStore.accounts.data}>
              <AccountsDropdown
                accounts={accounts}
                value={globalStore.currentlySelectedAccountUuid.data}
              />
            </Show>
          </div>
        </div>
      </nav>
    </Show>
  )
}

export default AppNavbar
