import { createStore } from "solid-js/store";

export interface IHash {
  [instanceId: string]: number;
}

export const [routesLastTab, setRoutesLastTab] = createStore<IHash>({});

export const getTabIndex = (tab: number) => {
  switch (tab) {
    case 0:
      return "overview";
    case 1:
      return "mods";
    case 2:
      return "resourcepacks";
    case 3:
      return "screenshots";
    case 4:
      return "versions";
    default:
      return "overview";
  }
};
