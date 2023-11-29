import { ManifestVersion, McType } from "@gd/core_module/bindings";
import { Match, Switch, createSignal } from "solid-js";

export const [mcVersions, setMcVersions] = createSignal<ManifestVersion[]>([]);
export const [mappedMcVersions, setMappedMcVersions] = createSignal<
  { label: string; key: string }[]
>([]);

export const mapMcTypeToColor = (
  type: McType,
  hasNoModloader: boolean | undefined
) => {
  return (
    <Switch>
      <Match when={type === "release"}>
        <span
          class="text-green-500"
          classList={{ "opacity-50": hasNoModloader }}
        >{`[${type}]`}</span>
      </Match>
      <Match when={type === "snapshot"}>
        <span
          class="text-yellow-500"
          classList={{ "opacity-50": hasNoModloader }}
        >{`[${type}]`}</span>
      </Match>
      <Match when={type === "old_alpha"}>
        <span
          class="text-purple-500"
          classList={{ "opacity-50": hasNoModloader }}
        >{`[${type}]`}</span>
      </Match>
      <Match when={type === "old_beta"}>
        <span
          class="text-red-500"
          classList={{ "opacity-50": hasNoModloader }}
        >{`[${type}]`}</span>
      </Match>
    </Switch>
  );
};
