import { Outlet, useLocation } from "@solidjs/router"
import ContentWrapper from "@/components/ContentWrapper"
import { Tab, TabList, Tabs } from "@gd/ui"
import { For, JSX } from "solid-js"
import { useGDNavigate } from "@/managers/NavigationManager"
import FeatureStatusBadge from "@/components/FeatureStatusBadge"
import { useTransContext } from "@gd/i18n"

export interface settingsItem {
  name: string | JSX.Element
  icon: string
  path: string
}

function Settings() {
  const location = useLocation()
  const navigator = useGDNavigate()
  const [t] = useTransContext()

  const settings: settingsItem[] = [
    {
      name: t("settings:General"),
      icon: "i-ri:home-gear-fill",
      path: "/settings"
    },
    {
      name: t("settings:Accounts"),
      icon: "i-ri:account-box-fill",
      path: "/settings/accounts"
    },
    {
      name: t("settings:Language"),
      icon: "i-ri:global-fill",
      path: "/settings/language"
    },
    {
      name: (
        <div class="relative flex gap-2 items-center">
          {"Appearance"}
          <div class="absolute -top-14 right-0">
            <FeatureStatusBadge type="beta" />
          </div>
        </div>
      ),
      icon: "i-ri:brush-fill",
      path: "/settings/appearance"
    },
    {
      name: t("settings:Java"),
      icon: "i-nonicons:java-16",
      path: "/settings/java"
    },
    {
      name: t("settings:custom_commands"),
      icon: "i-ri:terminal-fill",
      path: "/settings/custom-commands"
    },
    {
      name: t("settings:Privacy"),
      icon: "i-ri:shield-keyhole-fill",
      path: "/settings/privacy"
    },
    {
      name: t("settings:runtime_path"),
      icon: "i-ri-folder-fill",
      path: "/settings/runtime-path"
    }
  ]

  return (
    <>
      <ContentWrapper zeroPadding>
        <div class="w-full box-border sticky top-0 z-50 px-6">
          <Tabs
            orientation="horizontal"
            defaultIndex={settings.findIndex(
              (item) => item.path === location.pathname
            )}
          >
            <div class="h-26">
              <TabList>
                <For each={settings}>
                  {(item) => (
                    <Tab
                      onClick={() => {
                        navigator.navigate(item.path)
                      }}
                    >
                      <div class="flex flex-col gap-2 justify-center items-center">
                        <i class={"w-5 h-5 " + item.icon} />
                        <div class="text-center whitespace-nowrap">
                          {item.name}
                        </div>
                      </div>
                    </Tab>
                  )}
                </For>
              </TabList>
            </div>
          </Tabs>
        </div>
        <div class="pt-4 h-1 w-1" />
        <div class="px-6">
          <Outlet />
        </div>
        <div class="pb-4 h-1 w-1" />
      </ContentWrapper>
    </>
  )
}

export default Settings
