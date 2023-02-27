import { For } from "solid-js";
import SiderbarWrapper from "../wrapper";
import { Tab, TabList, Tabs } from "@gd/ui";
import { useGdNavigation } from "@/managers/NavigationManager";

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
    name: "Appearance",
    icon: "palette",
    path: "/settings/appearance",
  },
  {
    name: "Java",
    icon: "palette",
    path: "/settings/java",
  },
];

const Sidebar = () => {
  const navigate = useGdNavigation();

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
                  navigate?.navigate(item.path);
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
