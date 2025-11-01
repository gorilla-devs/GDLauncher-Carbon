import {
  AccountEntry,
  DeviceCode,
  EnrollmentError,
  EnrollmentStatus
} from "@gd/core_module/bindings"
import { RSPCError } from "@rspc/client"
import { CreateQueryResult } from "@tanstack/solid-query"

type RouteData = CreateQueryResult<EnrollmentStatus | null, RSPCError>

interface EventsCallbacks {
  onPolling?: (_info: DeviceCode) => void
  onWaitingForBrowser?: (_info: {
    authUrl: string
    redirectUri: string
    expiresAt: string
  }) => void
  onFail?: (_error: EnrollmentError) => void
  onError?: (_error: RSPCError | null) => void
  onComplete?: (_accountEntry: AccountEntry) => void
  onNeedsProfileCreation?: (_accessToken: string) => void
}

export const handleStatus = (
  routeData: RouteData | CreateQueryResult<any, RSPCError>,
  callbacks: EventsCallbacks
) => {
  if (routeData.isSuccess) {
    const data = routeData.data
    if (typeof data === "string" && !routeData.failureReason) return
    if (typeof data === "object" && data && "pollingCode" in data) {
      const info = data.pollingCode
      if (info) {
        return callbacks?.onPolling?.(info)
      }
    } else if (
      typeof data === "object" &&
      data &&
      "waitingForBrowser" in data
    ) {
      const info = data.waitingForBrowser
      if (info) {
        return callbacks?.onWaitingForBrowser?.({
          authUrl: info.authUrl,
          redirectUri: info.redirectUri,
          expiresAt: info.expiresAt
        })
      }
    } else if (
      typeof data === "object" &&
      data &&
      "needsProfileCreation" in data
    ) {
      const accessToken = data.needsProfileCreation.accessToken
      return callbacks?.onNeedsProfileCreation?.(accessToken)
    } else if (typeof data === "object" && data && "failed" in data) {
      const error = data.failed
      return callbacks?.onFail?.(error)
    } else if (typeof data === "object" && data && "complete" in data) {
      const complete = data.complete
      return callbacks?.onComplete?.(complete)
    }
  } else if (routeData.isError || routeData.failureReason) {
    return callbacks?.onError?.(routeData.error)
  }
}
