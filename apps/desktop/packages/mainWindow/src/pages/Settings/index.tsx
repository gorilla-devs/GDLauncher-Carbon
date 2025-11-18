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

  const settings = (): settingsItem[] => [
    {
      name: t("settings:_trn_general"),
      icon: "i-hugeicons:home-05",
      path: "/settings"
    },
    {
      name: t("accounts:_trn_accounts"),
      icon: "i-hugeicons:user-account",
      path: "/settings/accounts"
    },
    {
      name: t("settings:_trn_language"),
      icon: "i-hugeicons:globe",
      path: "/settings/language"
    },
    {
      name: (
        <div class="relative flex items-center gap-2">
          {"Appearance"}
          <div class="absolute -top-14 right-0">
            <FeatureStatusBadge type="beta" />
          </div>
        </div>
      ),
      icon: "i-hugeicons:paint-brush-01",
      path: "/settings/appearance"
    },
    {
      name: t("settings:_trn_java"),
      icon: "i-hugeicons:java",
      path: "/settings/java"
    },
    {
      name: t("settings:_trn_custom_commands"),
      icon: "i-hugeicons:computer-terminal-01",
      path: "/settings/custom-commands"
    },
    {
      name: t("settings:_trn_privacy"),
      icon: "i-hugeicons:security-lock",
      path: "/settings/privacy"
    },
    {
      name: t("java:_trn_runtime_path"),
      icon: "i-hugeicons:folder-01",
      path: "/settings/runtime-path"
    }
  ]

  return (
    <>
      <ContentWrapper zeroPadding>
        <div class="sticky top-0 z-50 box-border w-full px-6">
          <Tabs
            orientation="horizontal"
            defaultIndex={settings().findIndex(
              (item) => item.path === location.pathname
            )}
          >
            <div class="h-26">
              <TabList>
                <For each={settings()}>
                  {(item) => (
                    <Tab
                      onClick={() => {
                        navigator.navigate(item.path)
                      }}
                    >
                      <div class="flex flex-col items-center justify-center gap-2">
                        <i class={"w-5 h-5 " + item.icon} />
                        <div class="whitespace-nowrap text-center">
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
        <div class="h-1 w-1 pt-4" />
        <div class="px-6">
          <Outlet />
        </div>
        <div class="h-1 w-1 pb-4" />
      </ContentWrapper>
    </>
  )
}

export default Settings
