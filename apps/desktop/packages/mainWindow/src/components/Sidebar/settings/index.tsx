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
    icon: "settings",
    path: "/settings",
  },
  {
    name: "Language",
    icon: "language",
    path: "/settings/language",
  },
  {
    name: "Appearance",
    icon: "palette",
    path: "/settings/appearance",
  },
  {
    name: "Java",
    icon: "",
    path: "/settings/java",
  },
  {
    name: "Privacy",
    icon: "",
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
        paddingY="p-y-3.5"
      >
        <TabList>
          <For each={settings}>
            {(item) => (
              <Tab
                onClick={() => {
                  navigate(item.path);
                }}
              >
                {item.name}
              </Tab>
            )}
          </For>
        </TabList>
      </Tabs>
    </SiderbarWrapper>
  );
};

export default Sidebar;
