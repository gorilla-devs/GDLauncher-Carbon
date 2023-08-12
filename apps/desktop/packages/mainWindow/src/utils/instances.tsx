/* eslint-disable solid/no-innerhtml */
import {
  FeError,
  InvalidListInstance,
  LaunchState,
  ListInstanceStatus,
  Progress,
  FESubtask,
  FETaskId,
  UngroupedInstance,
  ValidListInstance,
  Modpack,
  CurseforgeModpack,
  ModrinthModpack,
  ModpackPlatform,
  CFFECategory,
  MRFECategory,
} from "@gd/core_module/bindings";
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg";
import CurseforgeLogo from "/assets/images/icons/curseforge_logo.svg";
import { Show, Switch, Match, createSignal } from "solid-js";
import { port } from "./rspcClient";

export const isListInstanceValid = (
  status: ListInstanceStatus
): status is { Valid: ValidListInstance } => {
  return "Valid" in status;
};

export const isListInstanceInValid = (
  status: ListInstanceStatus
): status is { Invalid: InvalidListInstance } => {
  return "Invalid" in status;
};

export const getValideInstance = (
  status: ListInstanceStatus
): ValidListInstance | undefined => {
  if (isListInstanceValid(status)) return status.Valid;
};

export const getInValideInstance = (
  status: ListInstanceStatus
): InvalidListInstance | undefined => {
  if (isListInstanceInValid(status)) return status.Invalid;
};

export const isListInstanceInvalid = (
  status: ListInstanceStatus
): status is { Invalid: InvalidListInstance } => {
  return "Invalid" in status;
};

export const getLaunchState = (
  launchState: LaunchState
):
  | { Preparing: FETaskId }
  | { Running: { start_time: string } }
  | undefined => {
  if (typeof launchState === "object" && "Preparing" in launchState) {
    return { Preparing: launchState.Preparing };
  } else if (typeof launchState === "object" && "Running" in launchState) {
    return { Running: launchState.Running };
  }
  return undefined;
};

export const isLaunchState = (input: any): input is LaunchState => {
  if (typeof input === "object") {
    // Check for the Preparing state
    if ("Preparing" in input) {
      return typeof input.Preparing === "number";
    }

    if ("Inactive" in input) {
      return input.Inactive.failed_task === null;
    }

    // Check for the Running state
    if ("Running" in input) {
      return (
        typeof input.Running === "object" &&
        "start_time" in input.Running &&
        typeof input.Running.start_time === "string" &&
        "log_id" in input.Running &&
        typeof input.Running.log_id === "number"
      );
    }
  }

  return false;
};

export const getPreparingState = (status: ListInstanceStatus | LaunchState) => {
  const launchState = isLaunchState(status);

  if (launchState) {
    if (typeof status === "object" && "Preparing" in status) {
      return status.Preparing;
    }
  } else {
    const isValidState = getValideInstance(status);
    if (
      isValidState &&
      isValidState.state &&
      typeof isValidState.state === "object" &&
      "Preparing" in isValidState.state
    ) {
      return isValidState.state.Preparing;
    }
  }
};

export const getInactiveState = (status: ListInstanceStatus | LaunchState) => {
  const launchState = isLaunchState(status);

  if (launchState) {
    if (typeof status === "object" && "Inactive" in status) {
      return status.Inactive.failed_task;
    }
  } else {
    const isValidState = getValideInstance(status);
    if (
      isValidState &&
      isValidState.state &&
      "Inactive" in isValidState.state
    ) {
      return isValidState.state.Inactive.failed_task;
    }
  }
};

export const isSubTaskDownload = (input: FESubtask): input is FESubtask => {
  return typeof input === "object" && "download" in input;
};

export const isSubTaskItem = (input: FESubtask): input is FESubtask => {
  return typeof input === "object" && "item" in input;
};

export const getRunningState = (status: ListInstanceStatus | LaunchState) => {
  const launchState = isLaunchState(status);

  if (launchState) {
    if (typeof status === "object" && "Running" in status) {
      return status.Running;
    }
  } else {
    const isValidState = getValideInstance(status);
    if (
      isValidState &&
      isValidState.state &&
      typeof isValidState.state === "object" &&
      "Running" in isValidState.state
    ) {
      return isValidState.state.Running;
    }
  }
};

export const isInstancePreparing = (
  launchState: LaunchState
): launchState is { Preparing: FETaskId } => {
  return typeof launchState === "object" && "Preparing" in launchState;
};

export const isInstanceRunning = (
  launchState: LaunchState
): launchState is { Running: { start_time: string; log_id: number } } => {
  return typeof launchState === "object" && "Running" in launchState;
};

export const isProgressKnown = (
  progress: Progress
): progress is { Known: number } => {
  return (progress as { Known: number }).Known !== undefined;
};

export const isProgressFailed = (
  progress: Progress
): progress is { Failed: FeError } => {
  return (progress as { Failed: FeError }).Failed !== undefined;
};

export const isModpackCurseforge = (
  modpack: Modpack
): modpack is { Curseforge: CurseforgeModpack } => {
  return "Curseforge" in modpack;
};

export const getCurseForgeData = (modpack: Modpack) => {
  if ("Curseforge" in modpack) return modpack.Curseforge;
};

export const isModpackModrinth = (
  modpack: Modpack
): modpack is { Modrinth: ModrinthModpack } => {
  return "Modrinth" in modpack;
};

export const getModrinthData = (modpack: Modpack) => {
  if ("Modrinth" in modpack) return modpack.Modrinth;
};

export const fetchImage = async (id: number) => {
  const imageUrl = `http://localhost:${port}/instance/instanceIcon?id=${id}`;
  const image = await fetch(imageUrl);

  const imageNotPresent = image.status === 204;

  if (!imageNotPresent) {
    return imageUrl;
  } else return "";
};

export const getUrlType = (url: string) => {
  return url.match(/^\/(modpacks|mods)\/\d+\/(curseforge|modrinth)(\/[^/]+)*$/)
    ? url.match(/mods/)
      ? "mods"
      : "modpacks"
    : null;
};

export interface InvalidInstanceType extends Omit<UngroupedInstance, "status"> {
  error?: InvalidListInstance;
}

export interface ValidInstanceType
  extends ValidListInstance,
    UngroupedInstance {
  error?: undefined;
  img: string;
}

export type Instance = InvalidInstanceType | ValidInstanceType;

export interface InstancesStore {
  [modloader: string]: UngroupedInstance[];
}

export const getModpackPlatformIcon = (platform: ModpackPlatform) => {
  switch (platform) {
    case "Curseforge":
      return CurseforgeLogo;
    case "Modrinth":
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

export const PlatformIcon = (props: { platform: ModpackPlatform }) => {
  return <img class="h-4 w-4" src={getModpackPlatformIcon(props.platform)} />;
};

export const getModpackPlatform = (modpack: Modpack) => {
  if ((modpack as { Curseforge: CurseforgeModpack }).Curseforge !== undefined) {
    return "Curseforge";
  } else if (
    (modpack as { Modrinth: ModrinthModpack }).Modrinth !== undefined
  ) {
    return "Modrinth";
  } else {
    return "Unknown";
  }
};

export const [importedInstances, setImportedInstances] = createSignal<number[]>(
  []
);
