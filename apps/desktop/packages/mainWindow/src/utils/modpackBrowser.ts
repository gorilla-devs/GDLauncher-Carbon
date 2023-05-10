import { createSignal } from "solid-js";

export const [modLoader, setModloader] = createSignal("any");
export const [selectedModpackCategory, setSelectedModpackCategory] =
  createSignal<string | number>("all");
