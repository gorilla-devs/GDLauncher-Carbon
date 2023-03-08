import {
  AccountEntry,
  DeviceCode,
  EnrollmentError,
  Procedures,
} from "@gd/core_module/bindings";
import { QueryObserverSuccessResult } from "@tanstack/solid-query";

type EnrollStatusResult = Extract<
  Procedures["queries"],
  { key: "account.enroll.getStatus" }
>["result"];

type RouteData = QueryObserverSuccessResult<EnrollStatusResult>;

type EventsCallbacks = {
  onPolling?: (_info: DeviceCode) => void;
  // TODO: handle errors
  onFail?: (_error: EnrollmentError) => void;
  onComplete?: (_accountEntry: AccountEntry) => void;
};

export const handleStatus = (
  routeData: RouteData,
  callbacks: EventsCallbacks
) => {
  const data = routeData.data;
  if (typeof data === "string") return;
  if (data && "PollingCode" in data) {
    const info = data.PollingCode;
    if (info) {
      callbacks?.onPolling?.(info);
    }
  } else if (data && "Failed" in data) {
    const error = data.Failed;
    callbacks?.onFail?.(error);
  } else if (data && "Complete" in data) {
    const complete = data.Complete;
    callbacks?.onComplete?.(complete);
  }
};
