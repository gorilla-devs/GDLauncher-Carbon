import { ManifestVersion } from "@gd/core_module/bindings";
import { createSignal } from "solid-js";

export const [mcVersions, setMcVersions] = createSignal<ManifestVersion[]>([]);
export const [mappedMcVersions, setMappedMcVersions] = createSignal<
  { label: string; key: string }[]
>([]);
