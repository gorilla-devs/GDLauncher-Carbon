import {
  InvalidListInstance,
  ListInstanceStatus,
  UngroupedInstance,
  ValidListInstance,
} from "@gd/core_module/bindings";
import { blobToBase64 } from "./helpers";
import { createStore } from "solid-js/store";
import { port } from "./rspcClient";

export const isListInstanceValid = (
  status: ListInstanceStatus
): status is { Valid: ValidListInstance } => {
  return "Valid" in status;
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
