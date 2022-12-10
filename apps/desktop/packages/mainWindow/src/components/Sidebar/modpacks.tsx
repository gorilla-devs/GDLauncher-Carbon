import { isSidebarOpened, toggleSidebar } from "@/stores/sidebar";
import { Show } from "solid-js";
import SiderbarWrapper from "./wrapper";

const Sidebar = () => {
  return (
    <SiderbarWrapper collapsable={false}>SIDEBAR modpacks</SiderbarWrapper>
  );
};

export default Sidebar;
