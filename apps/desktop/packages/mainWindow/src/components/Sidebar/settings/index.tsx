import { For } from "solid-js";
import SiderbarWrapper from "../wrapper";
import ListItem from "./ListItem";
import { Tab, TabList, Tabs } from "@gd/ui";
import { useNavigate } from "@solidjs/router";

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
];

const Sidebar = () => {
  const navigate = useNavigate();

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
