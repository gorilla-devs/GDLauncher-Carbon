import {
  InvalidListInstance,
  ListInstanceStatus,
  UngroupedInstance,
  ValidListInstance,
} from "@gd/core_module/bindings";

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
