import { For } from "solid-js";
import SiderbarWrapper from "../wrapper";
import ListItem from "./ListItem";
import { Tab, TabList, Tabs } from "@gd/ui";

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
  return (
    <SiderbarWrapper collapsable={false} noPadding>
      <Tabs orientation="vertical" variant="underline" gap={0}>
        <TabList>
          <For each={settings}>{(item) => <Tab>{item.name}</Tab>}</For>
        </TabList>
      </Tabs>
    </SiderbarWrapper>
  );
};

export default Sidebar;
