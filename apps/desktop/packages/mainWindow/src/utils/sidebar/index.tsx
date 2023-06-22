import { createSignal } from "solid-js";
import forgeIcon from "/assets/images/icons/forge.png";
import fabricIcon from "/assets/images/icons/fabric.png";
import quiltIcon from "/assets/images/icons/quilt.svg";
import vanillaIcon from "/assets/images/icons/vanilla.png";
import { ModLoaderType } from "@gd/core_module/bindings";

const [isSidebarOpened, setIsSidebarOpened] = createSignal(true);

export const toggleSidebar = () => {
  return setIsSidebarOpened(!isSidebarOpened());
};

const getModloaderIcon = (modloader?: ModLoaderType) => {
  switch (modloader) {
    case "Forge":
      return forgeIcon;
    case "Fabric":
      return fabricIcon;
    case "Quilt":
      return quiltIcon;
    default:
      return vanillaIcon;
  }
};

export { isSidebarOpened, getModloaderIcon };
