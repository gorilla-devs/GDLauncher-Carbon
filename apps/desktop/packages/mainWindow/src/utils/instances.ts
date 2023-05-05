import {
  FeError,
  InvalidListInstance,
  LaunchState,
  ListInstanceStatus,
  Progress,
  TaskId,
  UngroupedInstance,
  ValidListInstance,
} from "@gd/core_module/bindings";
import { blobToBase64 } from "./helpers";
import { port } from "./rspcClient";

export const isListInstanceValid = (
  status: ListInstanceStatus
): status is { Valid: ValidListInstance } => {
  return "Valid" in status;
};

export const isListInstanceInvalid = (
  status: ListInstanceStatus
): status is { Invalid: InvalidListInstance } => {
  return "Invalid" in status;
};

export const getLaunchState = (
  launchState: LaunchState
): { Preparing: TaskId } | { Running: { start_time: string } } | undefined => {
  if (typeof launchState === "object" && "Preparing" in launchState) {
    return { Preparing: launchState.Preparing };
  } else if (typeof launchState === "object" && "Running" in launchState) {
    return { Running: launchState.Running };
  }
  return undefined;
};

export const isInstancePreparing = (
  launchState: LaunchState
): launchState is { Preparing: TaskId } => {
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

export interface InvalidInstanceType extends Omit<UngroupedInstance, "status"> {
  error?: InvalidListInstance;
}

export interface ValidInstanceType
  extends ValidListInstance,
    UngroupedInstance {
  error?: undefined;
  img: string;
}

export const fetchImage = async (id: number) => {
  const image = await fetch(
    `http://localhost:${port}/instance/instanceIcon?id=${id}`
  );

  const imageNotPresent = image.status === 204;

  if (!imageNotPresent) {
    const blob = await image.blob();
    const b64 = (await blobToBase64(blob)) as string;
    return `data:image/png;base64, ${b64.substring(b64.indexOf(",") + 1)}`;
  } else return "";
};

export type Instance = InvalidInstanceType | ValidInstanceType;

export interface InstancesStore {
  [modloader: string]: UngroupedInstance[];
}
