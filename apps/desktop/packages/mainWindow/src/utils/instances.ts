import {
  FeError,
  InvalidListInstance,
  LaunchState,
  ListInstanceStatus,
  Progress,
  Subtask,
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
): { Preparing: TaskId } | { Running: { start_time: string } } | undefined => {
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

export const isSubTaskDownload = (input: Subtask): input is Subtask => {
  return typeof input === "object" && "download" in input;
};

export const isSubTaskItem = (input: Subtask): input is Subtask => {
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
