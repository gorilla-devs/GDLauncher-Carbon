/* eslint-disable solid/no-innerhtml */
import {
  InvalidListInstance,
  LaunchState,
  ListInstanceStatus,
  FESubtask,
  ListInstance,
  ValidListInstance,
  Modpack,
  CFFECategory,
  MRFECategory
} from "@gd/core_module/bindings";
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg";
import CurseforgeLogo from "/assets/images/icons/curseforge_logo.svg";
import { Show, Switch, Match, createSignal } from "solid-js";
import { port } from "./rspcClient";

export const isListInstanceInvalid = (status: ListInstanceStatus) => {
  return "Invalid" in status;
};

export const getLaunchState = (launchState: LaunchState | undefined) => {
  if (!launchState) return undefined;

  if (launchState.state === "preparing" || launchState.state === "running") {
    return launchState.value;
  }
  return undefined;
};

export const getPreparingState = (status: LaunchState | undefined) => {
  if (!status) return undefined;

  if (status.state === "preparing") {
    return status.value;
  }
};

export const getInactiveState = (status: LaunchState | undefined) => {
  if (!status) return undefined;

  if (status.state === "inactive") {
    return status.value.failed_task;
  }
};

export const isSubTaskDownload = (input: FESubtask): input is FESubtask => {
  return typeof input === "object" && "download" in input;
};

export const isSubTaskItem = (input: FESubtask): input is FESubtask => {
  return typeof input === "object" && "item" in input;
};

export const getRunningState = (status: LaunchState | undefined) => {
  if (!status) return undefined;

  if (status.state === "running") {
    return status.value;
  }
};

export const isInstanceDeleting = (status: LaunchState | undefined) => {
  if (!status) return false;

  return status.state === "deleting";
};

export const isInstancePreparing = (launchState: LaunchState) => {
  return launchState.state === "preparing";
};

export const isInstanceRunning = (launchState: LaunchState) => {
  return launchState.state === "running";
};

export const getInstanceImageUrl = (
  instanceId: string | number,
  rev: string | number
) => {
  return `http://127.0.0.1:${port}/instance/instanceIcon?id=${instanceId}&rev=${rev}`;
};

export const getModImageUrl = (
  instanceId: string,
  modId: string,
  platform: string | null
) => {
  return `http://127.0.0.1:${port}/instance/modIcon?instance_id=${instanceId}&mod_id=${modId}&platform=${platform}`;
};

export const getUrlType = (url: string) => {
  return url.match(
    /^\/(modpacks|mods)\/[a-zA-Z0-9]+\/(curseforge|modrinth)(\/[^/]+)*$/
  )
    ? url.match(/mods/)
      ? "mods"
      : "modpacks"
    : null;
};

export interface InvalidInstanceType extends Omit<ListInstance, "status"> {
  error?: InvalidListInstance;
}

export interface ValidInstanceType extends ValidListInstance, ListInstance {
  error?: undefined;
  img: string;
}

export type Instance = InvalidInstanceType | ValidInstanceType;

export interface InstancesStore {
  [modloader: string]: ListInstance[];
}

export const getModpackPlatformIcon = (
  modpackType: "curseforge" | "modrinth" | null | undefined
) => {
  switch (modpackType) {
    case "curseforge":
      return CurseforgeLogo;
    case "modrinth":
      return ModrinthLogo;
    default:
      return CurseforgeLogo;
  }
};

export const getCategoryIcon = (category: CFFECategory | MRFECategory) => {
  if ("iconUrl" in category) {
    return category.iconUrl;
  } else return category.icon;
};

export const CategoryIcon = (props: {
  category: CFFECategory | MRFECategory;
}) => {
  return (
    <Switch
      fallback={
        <div>
          <Show when={getCategoryIcon(props.category)}>
            <div
              class="w-4 h-4"
              innerHTML={getCategoryIcon(props.category) as string | undefined}
            />
          </Show>
        </div>
      }
    >
      <Match when={"iconUrl" in props.category}>
        <img
          class="h-4 w-4"
          src={getCategoryIcon(props.category) as string | undefined}
        />
      </Match>
    </Switch>
  );
};

export const PlatformIcon = (props: { modpack: "curseforge" | "modrinth" }) => {
  return <img class="h-4 w-4" src={getModpackPlatformIcon(props.modpack)} />;
};

export const getModpackPlatform = (modpack: Modpack) => {
  if (modpack.type === "curseforge") {
    return "Curseforge";
  } else if (modpack.type === "modrinth") {
    return "Modrinth";
  } else {
    return "Unknown";
  }
};

export const [importedInstances, setImportedInstances] = createSignal<number[]>(
  []
);
