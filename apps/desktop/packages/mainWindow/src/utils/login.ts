import {
  AccountEntry,
  DeviceCode,
  EnrollmentError,
} from "@gd/core_module/bindings";

type RouteData = {
  data:
    | "RequestingCode"
    | {
        PollingCode: DeviceCode;
      }
    | "QueryingAccount"
    | {
        Complete: AccountEntry;
      }
    | {
        Failed: EnrollmentError;
      };
};

type EventsCallbacks = {
  onPolling?: (_info: DeviceCode) => void;
  onFail?: (_error: string) => void;
  onComplete?: (_accountEntry: AccountEntry) => void;
};

export const handleStatus = (
  routeData: RouteData,
  callbacks: EventsCallbacks
) => {
  const data = routeData.data;
  if (typeof data === "string") return;
  if ("PollingCode" in data) {
    const info = data.PollingCode;
    if (info) {
      callbacks?.onPolling?.(info);
    }
  } else if ("Failed" in data) {
    const error = data.Failed;
    callbacks?.onFail?.(error);
  } else if ("Complete" in data) {
    const complete = data.Complete;
    callbacks?.onComplete?.(complete);
  }
};
