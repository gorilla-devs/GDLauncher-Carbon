import { Match, Show, Switch, createSignal } from "solid-js";
import forgeIcon from "/assets/images/icons/forge.png";
import fabricIcon from "/assets/images/icons/fabric.png";
import quiltIcon from "/assets/images/icons/quilt.svg";
import vanillaIcon from "/assets/images/icons/vanilla.png";
import {
  CFFECategory,
  CFFEModLoaderType,
  FEUnifiedSearchCategoryID,
  MRFECategory,
  MRFELoader
} from "@gd/core_module/bindings";
import { createStore } from "solid-js/store";

export const [isSidebarOpened, setIsSidebarOpened] = createSignal(true);
export const [supportedModloaders, setSupportedModloaders] = createStore<{
  modrinth: MRFELoader[];
  curseforge: CFFEModLoaderType[];
}>({
  modrinth: [],
  curseforge: []
});
export const [curseforgeCategories, setCurseforgeCategories] = createSignal<
  CFFECategory[]
>([]);
export const [modrinthCategories, setModrinthCategories] = createSignal<
  MRFECategory[]
>([]);

export const toggleSidebar = () => {
  return setIsSidebarOpened(!isSidebarOpened());
};

export const getCFModloaderIcon = (modloader?: CFFEModLoaderType) => {
  switch (modloader?.toString().toLowerCase()) {
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

export const getModloaderIcon = (category: CFFEModLoaderType | MRFELoader) => {
  const isCurseforge = typeof category === "string";

  if (isCurseforge) {
    return getCFModloaderIcon(category);
  } else {
    return (category as MRFELoader)?.icon;
  }
};

export const ModloaderIcon = (props: {
  modloader: CFFEModLoaderType | MRFELoader;
}) => {
  return (
    <Switch
      fallback={
        <>
          <Show when={getModloaderIcon(props.modloader)}>
            <div
              class="w-4 h-4"
              // eslint-disable-next-line solid/no-innerhtml
              innerHTML={getModloaderIcon(props.modloader)}
            />
          </Show>
        </>
      }
    >
      <Match when={typeof props.modloader === "string"}>
        <img class="h-4 w-4" src={getModloaderIcon(props.modloader)} />
      </Match>
    </Switch>
  );
};

export const getCategoryId = (searchCategory: FEUnifiedSearchCategoryID) => {
  if ("curseforge" in searchCategory) {
    return searchCategory.curseforge;
  } else if ("modrinth" in searchCategory) {
    return searchCategory.modrinth;
  }
};
