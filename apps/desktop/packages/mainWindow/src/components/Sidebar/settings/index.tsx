import { For } from "solid-js";
import SiderbarWrapper from "../wrapper";
import ListItem from "./ListItem";

export type settingsItem = {
  name: string;
  icon: string;
  path: string;
};

const settings: Array<settingsItem> = [
  {
    name: "General",
    icon: "settings",
    path: "/settings/general",
  },
  {
    name: "Appearance",
    icon: "palette",
    path: "/settings/appearance",
  },
];

const Sidebar = () => {
  return (
    <SiderbarWrapper collapsable={false}>
      <For each={settings}>{(item) => <ListItem item={item} />}</For>
    </SiderbarWrapper>
  );
};

export default Sidebar;
