import { createSignal } from "solid-js";
import forgeIcon from "/assets/images/icons/forge.png";
import fabricIcon from "/assets/images/icons/fabric.png";
import quiltIcon from "/assets/images/icons/quilt.svg";
import vanillaIcon from "/assets/images/icons/vanilla.png";
import { CFFEModLoaderType, MRFELoaderType } from "@gd/core_module/bindings";

const [isSidebarOpened, setIsSidebarOpened] = createSignal(true);

export const toggleSidebar = () => {
  return setIsSidebarOpened(!isSidebarOpened());
};

export type MRFELoaderTypeNoObject = Exclude<MRFELoaderType, { other: string }>;

const getModloaderIcon = (
  modloader?: CFFEModLoaderType | MRFELoaderTypeNoObject | string
) => {
  switch (modloader?.toLowerCase()) {
    case "forge":
      return forgeIcon;
    case "fabric":
      return fabricIcon;
    case "quilt":
      return quiltIcon;
    default:
      return vanillaIcon;
  }
};

export { isSidebarOpened, getModloaderIcon };
