import { createSignal } from "solid-js";
import forgeIcon from "/assets/images/icons/forge.png";
import vanillaIcon from "/assets/images/icons/vanilla.png";

export type ModloaderType = "forge" | "vanilla" | "fabric";

const [isSidebarOpened, setIsSidebarOpened] = createSignal(true);

export const toggleSidebar = () => {
  return setIsSidebarOpened(!isSidebarOpened());
};

const getModloaderIcon = (modloader: ModloaderType) => {
  switch (modloader) {
    case "vanilla":
      return vanillaIcon;
    case "forge":
      return forgeIcon;
    default:
      return vanillaIcon;
  }
};

export { isSidebarOpened, getModloaderIcon };
