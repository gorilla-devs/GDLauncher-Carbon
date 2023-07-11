import { For } from "solid-js";
import SiderbarWrapper from "../wrapper";
import { Tab, TabList, Tabs } from "@gd/ui";
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
    path: "/settings",
  },
  {
    name: "Language",
    icon: "i-ri:global-line",
    path: "/settings/language",
  },
  {
    name: "Appearance",
    icon: "i-ri:brush-line",
    path: "/settings/appearance",
  },
  {
    name: "Java",
    icon: "i-nonicons:java-16",
    path: "/settings/java",
  },
  {
    name: "Privacy",
    icon: "i-ri:shield-keyhole-line",
    path: "/settings/privacy",
  },
];

const Sidebar = () => {
  const navigate = useGDNavigate();

  return (
    <SiderbarWrapper collapsable={false} noPadding>
      <Tabs
        orientation="vertical"
        variant="underline"
        gap={0}
        paddingY="py-3.5"
      >
        <TabList>
          <For each={settings}>
            {(item) => (
              <Tab
                onClick={() => {
                  navigate(item.path);
                }}
              >
                <div class="flex gap-2 items-center">
                  <i class={"w-5 h-5 " + item.icon} />
                  <div>{item.name}</div>
                </div>
              </Tab>
            )}
          </For>
        </TabList>
      </Tabs>
    </SiderbarWrapper>
  );
};

export default Sidebar;
