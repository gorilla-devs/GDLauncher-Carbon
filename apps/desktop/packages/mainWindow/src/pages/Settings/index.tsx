import { Outlet, useLocation } from "@solidjs/router";
import ContentWrapper from "@/components/ContentWrapper";
import { Tab, TabList, Tabs } from "@gd/ui";
import { For } from "solid-js";
import { useGDNavigate } from "@/managers/NavigationManager";

export type settingsItem = {
  name: string;
  icon: string;
  path: string;
};

const settings: Array<settingsItem> = [
  {
    name: "General",
    icon: "i-ri:home-gear-fill",
    path: "/settings"
  },
  {
    name: "Accounts",
    icon: "i-ri:account-box-fill",
    path: "/settings/accounts"
  },
  {
    name: "Language",
    icon: "i-ri:global-fill",
    path: "/settings/language"
  },
  {
    name: "Appearance",
    icon: "i-ri:brush-fill",
    path: "/settings/appearance"
  },
  {
    name: "Java",
    icon: "i-nonicons:java-16",
    path: "/settings/java"
  },
  {
    name: "Custom Commands",
    icon: "i-ri:terminal-fill",
    path: "/settings/custom-commands"
  },
  {
    name: "Privacy",
    icon: "i-ri:shield-keyhole-fill",
    path: "/settings/privacy"
  },
  {
    name: "Runtime Path",
    icon: "i-ri-folder-fill",
    path: "/settings/runtime-path"
  }
];

function Settings() {
  const location = useLocation();
  const navigate = useGDNavigate();

  return (
    <>
      <ContentWrapper>
        <div class="w-full box-border sticky top-0 z-50">
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
                        navigate(item.path);
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
        <Outlet />
        <div class="pb-4 h-1 w-1" />
      </ContentWrapper>
    </>
  );
}

export default Settings;
