import { createMemo } from "solid-js"
import type { CreateQueryResult } from "@tanstack/solid-query"

/**
 * Hook for managing enrollment status with convenient accessors
 *
 * Provides type-safe access to different enrollment status states
 */
export function useEnrollmentStatus(
  enrollmentStatus: CreateQueryResult<any, any>
) {
  // Check if we're in device code polling state
  const isPollingCode = createMemo(() => {
    const status = enrollmentStatus.data
    return status && typeof status === "object" && status !== null && "pollingCode" in status
  })

  // Check if we're waiting for browser authentication
  const isWaitingForBrowser = createMemo(() => {
    const status = enrollmentStatus.data
    return status && typeof status === "object" && status !== null && "waitingForBrowser" in status
  })

  // Check if profile creation is needed
  const needsProfileCreation = createMemo(() => {
    const status = enrollmentStatus.data
    return status && typeof status === "object" && status !== null && "needsProfileCreation" in status
  })

  // Check if enrollment is complete
  const isComplete = createMemo(() => {
    const status = enrollmentStatus.data
    return status && typeof status === "object" && status !== null && "complete" in status
  })

  // Check if enrollment has failed
  const hasFailed = createMemo(() => {
    const status = enrollmentStatus.data
    return status && typeof status === "object" && status !== null && "failed" in status
  })

  // Get device code info
  const deviceCode = createMemo(() => {
    const status = enrollmentStatus.data
    if (status && typeof status === "object" && status !== null && "pollingCode" in status) {
      return status.pollingCode
    }
    return null
  })

  // Get browser auth info
  const browserAuthInfo = createMemo(() => {
    const status = enrollmentStatus.data
    if (status && typeof status === "object" && status !== null && "waitingForBrowser" in status) {
      return status.waitingForBrowser
    }
    return null
  })

  // Get profile creation access token
  const profileCreationToken = createMemo(() => {
    const status = enrollmentStatus.data
    if (status && typeof status === "object" && status !== null && "needsProfileCreation" in status) {
      return status.needsProfileCreation.accessToken
    }
    return null
  })

  // Get error info
  const errorInfo = createMemo(() => {
    const status = enrollmentStatus.data
    if (status && typeof status === "object" && status !== null && "failed" in status) {
      return status.failed
    }
    return null
  })

  // Get current step name for debugging/logging
  const currentStep = createMemo(() => {
    const status = enrollmentStatus.data
    if (!status) return "idle"

    // Handle case where status might be a string or non-object
    if (typeof status !== "object" || status === null) {
      return String(status)
    }

    if ("refreshingMsAuth" in status) return "refreshingMsAuth"
    if ("requestingCode" in status) return "requestingCode"
    if ("pollingCode" in status) return "pollingCode"
    if ("waitingForBrowser" in status) return "waitingForBrowser"
    if ("mcLogin" in status) return "mcLogin"
    if ("xboxAuth" in status) return "xboxAuth"
    if ("mcEntitlements" in status) return "mcEntitlements"
    if ("mcProfile" in status) return "mcProfile"
    if ("needsProfileCreation" in status) return "needsProfileCreation"
    if ("complete" in status) return "complete"
    if ("failed" in status) return "failed"

    return "unknown"
  })

  // Check if currently in an active enrollment process
  const isEnrolling = createMemo(() => {
    return (
      enrollmentStatus.data &&
      !isComplete() &&
      !hasFailed() &&
      !needsProfileCreation()
    )
  })

  return {
    // Status checks
    isPollingCode,
    isWaitingForBrowser,
    needsProfileCreation,
    isComplete,
    hasFailed,
    isEnrolling,

    // Data accessors
    deviceCode,
    browserAuthInfo,
    profileCreationToken,
    errorInfo,
    currentStep,

    // Raw status
    raw: enrollmentStatus
  }
}
