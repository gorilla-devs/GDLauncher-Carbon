import { FECategory } from "@gd/core_module/bindings";
import { createSignal } from "solid-js";
import { createStore } from "solid-js/store";

export interface Categories extends FECategory {
  selected: boolean;
}

export const [modLoader, setModloader] = createSignal("any");
export const [modpacksCategories, setModpacksCategories] = createStore<
  Categories[]
>([]);
