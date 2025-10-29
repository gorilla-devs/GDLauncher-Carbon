import { useLocation, useMatch } from "@solidjs/router"
import { Match, Show, Switch, createMemo } from "solid-js"
import GDLauncherWideLogo from "/assets/images/gdlauncher_wide_logo_blue.svg"
import {
  Tab,
  TabList,
  Tabs,
  Tooltip,
  Button,
  TooltipContent,
  TooltipTrigger,
  AnimatedIcon
} from "@gd/ui"
import { useGDNavigate } from "@/managers/NavigationManager"
import { AccountsDropdown } from "./AccountsDropdown"
import { AccountStatus, AccountType } from "@gd/core_module/bindings"
import { port } from "@/utils/rspcClient"
import updateAvailable, {
  updateDownloaded,
  updateProgress
} from "@/utils/updater"
import { Trans } from "@gd/i18n"
import { useModal } from "@/managers/ModalsManager"
import { useGlobalStore } from "./GlobalStoreContext"
import { EnhancedSearchBar } from "./EnhancedSearchBar"
import { getAccountImageUuid } from "@/utils/showcaseHelpers"

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

  const isLogin = useMatch(() => "/")
  const isSettings = useMatch(() => "/settings")
  const isSettingsNested = useMatch(() => "/settings/*")
  const isNews = useMatch(() => "/news/*")
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
            icon: `http://127.0.0.1:${port}/account/headImage?uuid=${getAccountImageUuid(account)}`,
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
          <div
            class="bg-darkSlate-700 -z-1 absolute left-0 top-0 h-full w-full scale-75 rounded-md opacity-0 transition-[transform,opacity] duration-150 ease-[cubic-bezier(.4,0,.2,1)] group-hover:scale-100 group-hover:opacity-100"
            style={{
              "will-change": "transform, opacity"
            }}
          />
          <div class="flex items-center gap-2 px-2">
            <AnimatedIcon
              icon="i-hugeicons:arrow-left-01"
              size="h-6 w-6"
              class="transition-[transform,opacity] duration-200 ease-[cubic-bezier(.4,0,.2,1)]"
              classList={{
                "opacity-0 -translate-x-4": location.pathname === "/library"
              }}
              style={{
                "will-change": "transform, opacity"
              }}
            />
            <img
              src={GDLauncherWideLogo}
              class="h-9 max-w-none transition-transform duration-200 ease-[cubic-bezier(.4,0,.2,1)]"
              classList={{
                "-translate-x-4": location.pathname === "/library"
              }}
              style={{
                "will-change": "transform"
              }}
            />
          </div>
        </div>
        <div class="flex w-full items-center justify-center gap-4">
          <EnhancedSearchBar />
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
            <AnimatedIcon icon="i-hugeicons:add-01" class="flex" />
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
                    <AnimatedIcon
                      icon="i-hugeicons:settings-01"
                      size="text-2xl"
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
                    <AnimatedIcon
                      icon="i-hugeicons:news"
                      size="text-2xl"
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
                        <AnimatedIcon
                          icon="i-hugeicons:download-02"
                          size="text-2xl"
                          class="text-green-500"
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
                accounts={accounts()}
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
