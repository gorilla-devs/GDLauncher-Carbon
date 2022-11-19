import Library from "@/components/Sidebar/contents/library";
import Modpacks from "@/components/Sidebar/contents/modpacks";

export const routes = [
  { label: "Library", href: "/library", sidebarContent: Library },
  { label: "Modpacks", href: "/modpacks", sidebarContent: Modpacks },
];
