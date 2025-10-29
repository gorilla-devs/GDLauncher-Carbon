import { createSignal, createEffect } from "solid-js"
import { rspc } from "@/utils/rspcClient"
import { handleStatus } from "@/utils/login"
import { parseError } from "@/utils/helpers"
import { toast } from "@gd/ui"
import type { DeviceCodeObjectType } from "../index"

/**
 * Helper function to get a unique key for enrollment status
 * Used for change detection to avoid calling callbacks on every poll
 */
function getStatusKey(status: any): string {
  if (!status) return "null"
  if (typeof status !== "object") return String(status)

  // Generate a key based on the status variant
  if ("pollingCode" in status) return `pollingCode:${status.pollingCode.userCode}`
  if ("waitingForBrowser" in status) return `waitingForBrowser:${status.waitingForBrowser.redirectUri}`
  if ("needsProfileCreation" in status) return `needsProfileCreation:${status.needsProfileCreation.accessToken}`
  if ("complete" in status) return `complete:${status.complete.uuid}`
  if ("failed" in status) return `failed:${JSON.stringify(status.failed)}`
  if ("refreshingMsAuth" in status) return "refreshingMsAuth"
  if ("requestingCode" in status) return "requestingCode"
  if ("mcLogin" in status) return "mcLogin"
  if ("xboxAuth" in status) return "xboxAuth"
  if ("mcEntitlements" in status) return "mcEntitlements"
  if ("mcProfile" in status) return "mcProfile"

  return "unknown"
}

/**
 * Core authentication flow hook
 *
 * Manages:
 * - Enrollment status polling
 * - Device code tracking
 * - Profile creation state
 * - Authentication mutations
 * - Step transitions based on enrollment status
 */
export function useAuthFlow(props: {
  onPollingCode: (deviceCode: DeviceCodeObjectType) => void
  onWaitingForBrowser: (info: { authUrl: string; redirectUri: string; expiresAt: string }) => void
  onNeedsProfileCreation: (accessToken: string) => void
  onError: (error: any) => void
  onComplete: () => Promise<void>
}) {
  const [loadingButton, setLoadingButton] = createSignal(false)
  const [profileAccessToken, setProfileAccessToken] = createSignal<string>("")
  const [previousStatusKey, setPreviousStatusKey] = createSignal<string>("")
  const [isEnrolling, setIsEnrolling] = createSignal(false)

  let lastEnrollmentCall = 0
  const ENROLLMENT_DEBOUNCE_MS = 1000

  const rspcContext = rspc.useContext()

  // Enrollment mutations
  const enrollBeginMutation = rspc.createMutation(() => ({
    mutationKey: ["account.enroll.begin"]
  }))

  // @ts-ignore - Type will be generated after backend rebuild
  const enrollBeginBrowserMutation = rspc.createMutation(() => ({
    mutationKey: ["account.enroll.beginBrowser"]
  }))

  const enrollCancelMutation = rspc.createMutation(() => ({
    mutationKey: ["account.enroll.cancel"]
  }))

  const enrollFinalizeMutation = rspc.createMutation(() => ({
    mutationKey: ["account.enroll.finalize"]
  }))

  const enrollResumeMutation = rspc.createMutation(() => ({
    mutationKey: ["account.enroll.resume"]
  }))

  // Enrollment status query (polling)
  const enrollmentStatus = rspc.createQuery(() => ({
    queryKey: ["account.enroll.getStatus"],
    refetchInterval: 500 // Poll every 500ms
  }))

  // Handle enrollment status changes
  // Only process status when it actually changes (not on every 500ms poll)
  createEffect(() => {
    const currentStatus = enrollmentStatus.data
    const currentKey = getStatusKey(currentStatus)

    // Skip if status hasn't changed
    if (currentKey === previousStatusKey()) {
      return
    }

    // Update previous status key
    setPreviousStatusKey(currentKey)

    // Process the new status
    handleStatus(enrollmentStatus, {
      onPolling: async (info) => {
        props.onPollingCode({
          userCode: info.userCode,
          link: info.verificationUri,
          expiresAt: info.expiresAt
        })
        setLoadingButton(false)
      },
      onWaitingForBrowser: async (info) => {
        props.onWaitingForBrowser(info)
        setLoadingButton(false)
      },
      onNeedsProfileCreation: (accessToken) => {
        setProfileAccessToken(accessToken)
        props.onNeedsProfileCreation(accessToken)
        setLoadingButton(false)
      },
      async onError(error) {
        if (error) toast.error(parseError(error))
        props.onError(error)
        setLoadingButton(false)
      },
      async onComplete() {
        await props.onComplete()
      }
    })
  })

  // Start enrollment with device code
  const beginEnrollment = async () => {
    const now = Date.now()
    const timeSinceLastCall = now - lastEnrollmentCall

    // Guard 1: Debounce - prevent rapid-fire calls
    if (timeSinceLastCall < ENROLLMENT_DEBOUNCE_MS) {
      console.warn(`[Auth] Enrollment called too quickly (${timeSinceLastCall}ms since last call), debouncing`)
      return
    }

    // Guard 2: Lock - prevent concurrent execution
    const enrollingState = isEnrolling()
    console.log('[Auth] beginEnrollment called, current isEnrolling:', enrollingState)

    if (enrollingState) {
      console.warn('[Auth] Enrollment already in progress, ignoring duplicate call')
      return
    }

    lastEnrollmentCall = now
    console.log('[Auth] Setting enrollment lock')
    setIsEnrolling(true)
    setLoadingButton(true)

    try {
      // Backend automatically cancels any existing enrollment
      console.log('[Auth] Starting new device code enrollment')
      await enrollBeginMutation.mutateAsync(undefined)
    } catch (error: any) {
      toast.error(parseError(error))
      setLoadingButton(false)
    } finally {
      setIsEnrolling(false)
    }
  }

  // Start enrollment with browser OAuth
  const beginBrowserEnrollment = async (openBrowser = true) => {
    const now = Date.now()
    const timeSinceLastCall = now - lastEnrollmentCall

    // Guard 1: Debounce - prevent rapid-fire calls
    if (timeSinceLastCall < ENROLLMENT_DEBOUNCE_MS) {
      console.warn(`[Auth] Browser enrollment called too quickly (${timeSinceLastCall}ms since last call), debouncing`)
      return
    }

    // Guard 2: Lock - prevent concurrent execution
    if (isEnrolling()) {
      console.warn('[Auth] Enrollment already in progress, ignoring duplicate call')
      return
    }

    lastEnrollmentCall = now
    setIsEnrolling(true)
    setLoadingButton(true)

    try {
      // Backend automatically cancels any existing enrollment
      console.log('[Auth] Starting new browser enrollment')
      // @ts-ignore - Type will be generated after backend rebuild
      await enrollBeginBrowserMutation.mutateAsync(openBrowser)
    } catch (error: any) {
      toast.error(parseError(error))
      setLoadingButton(false)
    } finally {
      setIsEnrolling(false)
    }
  }

  // Cancel enrollment
  const cancelEnrollment = async () => {
    try {
      await enrollCancelMutation.mutateAsync(undefined)
    } catch (error: any) {
      // Silently fail - error is expected if no enrollment is active
      console.log('[Auth] Cancel enrollment failed:', error)
      throw error // Re-throw so caller can handle if needed
    }
  }

  // Finalize enrollment (called after successful auth)
  const finalizeEnrollment = async () => {
    try {
      await enrollFinalizeMutation.mutateAsync(undefined)

      // Get the newly created account
      const activeUuid = await rspcContext.client.query([
        "account.getActiveUuid"
      ])

      if (!activeUuid) {
        throw new Error("No active uuid after enrollment")
      }

      return activeUuid
    } catch (error: any) {
      toast.error(parseError(error))
      throw error
    }
  }

  // Resume enrollment (for profile creation step)
  const resumeEnrollment = async () => {
    try {
      await enrollResumeMutation.mutateAsync(undefined)
    } catch (error: any) {
      toast.error(parseError(error))
      throw error
    }
  }

  return {
    // State
    loadingButton,
    profileAccessToken,
    enrollmentStatus,

    // Mutations
    enrollBeginMutation,
    enrollBeginBrowserMutation,
    enrollCancelMutation,
    enrollFinalizeMutation,
    enrollResumeMutation,

    // Methods
    beginEnrollment,
    beginBrowserEnrollment,
    cancelEnrollment,
    finalizeEnrollment,
    resumeEnrollment,

    // Setters (for external control)
    setLoadingButton,
    setProfileAccessToken
  }
}
