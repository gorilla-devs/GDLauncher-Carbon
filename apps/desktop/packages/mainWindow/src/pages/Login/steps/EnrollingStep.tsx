import { Trans, useTransContext } from "@gd/i18n"
import {
  Button,
  Progress,
  toast,
  Popover,
  PopoverContent,
  PopoverTrigger,
  CopyText,
  Skeleton
} from "@gd/ui"
import {
  Show,
  Switch,
  Match,
  createSignal,
  createEffect,
  onCleanup
} from "solid-js"
import QRCode from "qrcode"
import { msToMinutes, msToSeconds, parseTwoDigitNumber } from "@/utils/helpers"
import GateAnimationRiveWrapper from "@/utils/GateAnimationRiveWrapper"
import GateAnimation from "../../../gate_animation.riv"
import { useFlow } from "../flow/FlowContext"
import type { AuthStep } from "../flow/types"
import { rspc } from "@/utils/rspcClient"

/**
 * EnrollingStep
 *
 * Authentication enrollment step that handles both browser OAuth and device code flows.
 * Shows loading states, countdown timers, and enrollment progress.
 * Polls backend for status updates and handles various enrollment stages.
 */

interface EnrollingStepProps {
  step: Extract<AuthStep, { type: "enrolling" }>
}

export function EnrollingStep(props: EnrollingStepProps) {
  const flow = useFlow()
  const [t] = useTransContext()
  const [loading, setLoading] = createSignal(false)

  let qrCanvasRef: HTMLCanvasElement | undefined

  const method = () => props.step.method

  // Poll enrollment status from backend
  const enrollmentStatus = rspc.createQuery(() => ({
    queryKey: ["account.enroll.getStatus"],
    refetchInterval: 1000, // Poll every second
    refetchIntervalInBackground: true
  }))

  const status = () => enrollmentStatus.data

  // Check if polling code data is available
  const hasPollingCode = () => {
    const s = status()
    return s && typeof s === "object" && "pollingCode" in s
  }

  // Timer states - extract expiry based on status variant
  const expiresAtMs = () => {
    const s = status()
    if (!s || typeof s === "string") return 0

    if ("pollingCode" in s) {
      return new Date(s.pollingCode.expiresAt).getTime() - Date.now()
    }
    if ("waitingForBrowser" in s) {
      return new Date(s.waitingForBrowser.expires_at).getTime() - Date.now()
    }
    return 0
  }

  const minutes = () => msToMinutes(expiresAtMs())
  const seconds = () => msToSeconds(expiresAtMs())
  const [countDown, setCountDown] = createSignal(
    `${minutes()}:${parseTwoDigitNumber(seconds())}`
  )
  const [expired, setExpired] = createSignal(false)

  // Reset expired state when expiresAt changes
  createEffect(() => {
    const timeRemaining = expiresAtMs()
    if (timeRemaining > 0) {
      setExpired(false)
    }
  })

  // Update countdown timer
  const updateExpireTime = () => {
    const remaining = expiresAtMs()
    if (remaining > 0 && minutes() <= 0 && seconds() <= 0) {
      setExpired(true)
    } else {
      setCountDown(`${minutes()}:${parseTwoDigitNumber(seconds())}`)
    }
  }

  // Set up countdown interval
  const interval = setInterval(() => {
    updateExpireTime()
  }, 1000)

  onCleanup(() => {
    clearInterval(interval)
  })

  // Handle switch to device code
  const handleSwitchToDeviceCode = async () => {
    setLoading(true)
    try {
      // Cancel current enrollment
      await flow.cancelEnrollment()

      // Start device code enrollment
      await flow.startEnrollment("device-code")

      await flow.goToStep({
        type: "enrolling",
        method: "device-code"
      })
    } finally {
      setLoading(false)
    }
  }

  // Handle manual browser open (browser method only)
  const openBrowser = () => {
    const s = status()
    if (s && typeof s === "object" && "waitingForBrowser" in s) {
      window.openExternalLink(s.waitingForBrowser.auth_url)
      toast.success(t("auth:_trn_login.browser_opened_toast"))
    }
  }

  // Generate QR code when device code is available
  createEffect(async () => {
    const s = status()
    if (
      s &&
      typeof s === "object" &&
      "pollingCode" in s &&
      qrCanvasRef &&
      method() === "device-code"
    ) {
      try {
        await QRCode.toCanvas(qrCanvasRef, s.pollingCode.verificationUri, {
          width: 120,
          margin: 2,
          color: {
            dark: "#000000",
            light: "#FFFFFF"
          }
        })
      } catch (err) {
        console.error("Error generating QR code:", err)
      }
    }
  })

  // Handle enrollment status updates from backend
  createEffect(() => {
    const backendStatus = enrollmentStatus.data

    if (!backendStatus) return

    // Handle successful completion
    if (typeof backendStatus === "object" && "complete" in backendStatus) {
      // Enrollment successful - save to database, then check GDL account
      flow
        .finalizeEnrollment()
        .then(() => flow.checkGDLAccount())
        .then((gdlState) => {
          flow.goToStep({ type: "gdl-account", gdlAccount: gdlState })
        })
        .catch((error) => {
          console.error("[EnrollingStep] Failed to finalize enrollment:", error)
          flow.goToStep({
            type: "error",
            message: "Failed to save account. Please try again.",
            canRetry: true
          })
        })
      return
    }

    // Handle profile creation needed
    if (
      typeof backendStatus === "object" &&
      "needsProfileCreation" in backendStatus
    ) {
      flow.goToStep({
        type: "profile-creation",
        accessToken: backendStatus.needsProfileCreation.access_token
      })
      return
    }

    // Handle error states
    if (typeof backendStatus === "object" && "failed" in backendStatus) {
      toast.error("Authentication Error", {
        description:
          backendStatus.failed.description ||
          t("errors:_trn_error.enrollment_failed")
      })
      return
    }

    // Update status display based on backend stage
    // The backend status will be one of the enrollment stages:
    // "refreshingMSAuth", "requestingCode", "pollingCode", "waitingForBrowser",
    // "xboxAuth", "mcLogin", "mcProfile", "mcentitlements"
  })

  return (
    <div class="relative flex w-full flex-1 flex-col items-center justify-center gap-8 p-6 text-center">
      {/* Animation */}
      <GateAnimationRiveWrapper width={80} height={80} src={GateAnimation} />

      {/* Browser Method */}
      <Show when={method() === "browser"}>
        {/* Expired state */}
        <Show when={expired()}>
          <div class="flex flex-col items-center gap-4">
            <p class="text-lightSlate-50 text-lg font-semibold">
              <Trans key="auth:_trn_login.session_expired_title" />
            </p>
            <p class="text-lightSlate-700 text-sm">
              <Trans key="auth:_trn_login.session_expired_message" />
            </p>
          </div>
        </Show>

        {/* Active waiting state */}
        <Show when={!expired()}>
          <div class="flex flex-col items-center gap-4">
            {/* Main message */}
            <div class="flex flex-col gap-2">
              <p class="text-lightSlate-50 text-lg font-semibold">
                <Trans key="auth:_trn_login.waiting_for_browser_auth" />
              </p>
              <p class="text-lightSlate-700 text-sm">
                <Trans key="auth:_trn_login.complete_signin_browser" />
              </p>
            </div>

            {/* Countdown */}
            <p class="text-lightSlate-700 text-sm">
              <span class="text-lightSlate-500 mr-1">{countDown()}</span>
              <Trans key="auth:_trn_login.before_expiring" />
            </p>

            {/* Manual fallback options */}
            <div class="border-darkSlate-600 flex flex-col items-center gap-2 border-t pt-4">
              <p class="text-lightSlate-600 text-xs">
                <Trans key="auth:_trn_login.browser_didnt_open" />
              </p>
              <Button size="small" type="secondary" onClick={openBrowser}>
                <div class="i-hugeicons:link-square-02" />
                <Trans key="auth:_trn_login.open_browser_manually" />
              </Button>
            </div>
          </div>
        </Show>
      </Show>

      {/* Device Code Method */}
      <Show when={method() === "device-code"}>
        {/* Help popover */}
        <div class="absolute right-4 top-4">
          <Popover>
            <PopoverTrigger>
              <div class="text-lightSlate-700 hover:text-lightSlate-50 transition-color flex items-center text-sm duration-75">
                <div>
                  <Trans key="auth:_trn_login.need_help" />
                </div>
                <div class="i-hugeicons:help-circle ml-2 h-4 w-4" />
              </div>
            </PopoverTrigger>
            <PopoverContent>
              <div class="max-w-100 px-4 pb-6 text-sm">
                <h3>
                  <Trans key="auth:_trn_login.troubles_logging_in" />
                </h3>
                <div class="pb-8 text-sm">
                  <Trans key="auth:_trn_login.link_not_working_help" />
                </div>
                <div
                  class="text-lightSlate-600 hover:text-lightSlate-50 flex items-center gap-2"
                  onClick={() => {
                    const s = status()
                    if (s && typeof s === "object" && "pollingCode" in s) {
                      const link = s.pollingCode.verificationUri
                      navigator.clipboard.writeText(link)
                      toast.success("The link has been copied")
                    }
                  }}
                >
                  <div class="i-hugeicons:link-01 h-4 w-4" />
                  <div>
                    {(() => {
                      const s = status()
                      return s && typeof s === "object" && "pollingCode" in s
                        ? s.pollingCode.verificationUri
                        : ""
                    })()}
                  </div>
                </div>
              </div>
            </PopoverContent>
          </Popover>
        </div>

        {/* Steps container */}
        <div class="flex flex-col items-center gap-3">
          {/* Step 1: Open the link */}
          <Show when={!expired()}>
            <div class="bg-darkSlate-800 flex flex-col items-center gap-3 rounded-xl px-6 py-4">
              <p class="text-lightSlate-200 m-0 text-base font-semibold">
                <Trans key="auth:_trn_login.step_1_open_link" />
              </p>

              {/* QR Code */}
              <div class="rounded-lg bg-white p-2">
                <Show
                  when={hasPollingCode()}
                  fallback={<Skeleton class="h-[120px] w-[120px] rounded" />}
                >
                  <canvas
                    ref={qrCanvasRef}
                    width="120"
                    height="120"
                    class="rounded"
                  />
                </Show>
              </div>
              <p class="text-lightSlate-500 m-0 text-xs">
                <Trans key="auth:_trn_login.scan_to_open" />
              </p>

              {/* Divider */}
              <div class="flex w-full max-w-xs items-center gap-3">
                <div class="bg-darkSlate-600 h-px flex-1" />
                <span class="text-lightSlate-600 text-xs">
                  <Trans key="general:_trn_or" />
                </span>
                <div class="bg-darkSlate-600 h-px flex-1" />
              </div>

              {/* Manual URL */}
              <Show
                when={hasPollingCode()}
                fallback={
                  <div class="bg-darkSlate-700 flex items-center rounded px-3 py-2 gap-2">
                    <Skeleton class="h-5 w-48" />
                    <Skeleton class="h-9 w-16 rounded-md" />
                  </div>
                }
              >
                <CopyText
                  value={(() => {
                    const s = status()
                    return s && typeof s === "object" && "pollingCode" in s
                      ? s.pollingCode.verificationUri
                      : ""
                  })()}
                  onCopy={() => toast.success(t("auth:_trn_login.link_copied"))}
                />
              </Show>
            </div>
          </Show>

          {/* Step 2: Enter the code */}
          <div class="bg-darkSlate-800 flex flex-col items-center gap-2 rounded-xl px-6 py-4">
            <Show when={!expired()}>
              <p class="text-lightSlate-200 m-0 text-base font-semibold">
                <Trans key="auth:_trn_login.step_2_enter_code" />
              </p>
            </Show>

            <Show
              when={!expired()}
              fallback={
                <span class="text-lightSlate-700 text-2xl font-bold tracking-wider">
                  {(() => {
                    const s = status()
                    return s && typeof s === "object" && "pollingCode" in s
                      ? s.pollingCode.userCode
                      : ""
                  })()}
                </span>
              }
            >
              <Show
                when={hasPollingCode()}
                fallback={
                  <div class="bg-darkSlate-700 flex items-center rounded px-6 py-3 gap-3">
                    <Skeleton class="h-8 w-32" />
                    <Skeleton class="h-9 w-20 rounded-md" />
                  </div>
                }
              >
                <CopyText
                  size="large"
                  value={(() => {
                    const s = status()
                    return s && typeof s === "object" && "pollingCode" in s
                      ? s.pollingCode.userCode
                      : ""
                  })()}
                  onCopy={() =>
                    toast.success(t("general:_trn_general_copied_to_clipboard"))
                  }
                />
              </Show>
            </Show>

            <Show when={expired()}>
              <p class="text-sm text-red-500">
                <Trans key="auth:_trn_login.code_expired_message" />
              </p>
            </Show>

            <Show when={!expired()}>
              <p class="text-lightSlate-700 m-0 text-sm">
                <span class="text-lightSlate-500 mr-1">{countDown()}</span>
                <Trans key="auth:_trn_login.before_expiring" />
              </p>
            </Show>
          </div>
        </div>
      </Show>

      {/* Loading progress with stages (both methods) - always rendered to prevent layout shift */}
      <div
        class="flex flex-col items-center gap-2 transition-opacity duration-200"
        classList={{
          "opacity-0 pointer-events-none": !(
            enrollmentStatus.data &&
            typeof enrollmentStatus.data === "string" &&
            !expired()
          ),
          "opacity-100": !!(
            enrollmentStatus.data &&
            typeof enrollmentStatus.data === "string" &&
            !expired()
          )
        }}
      >
        <span class="text-lightSlate-700 text-xs">
          <Switch>
            <Match
              when={
                enrollmentStatus.data &&
                typeof enrollmentStatus.data === "object" &&
                "waitingForBrowser" in enrollmentStatus.data
              }
            >
              <Trans key="auth:_trn_login.waiting_for_browser_confirmation" />
            </Match>
            <Match
              when={
                enrollmentStatus.data &&
                typeof enrollmentStatus.data === "object" &&
                "pollingCode" in enrollmentStatus.data
              }
            >
              <Trans key="auth:_trn_login.polling_microsoft_auth" />
            </Match>
            <Match when={enrollmentStatus.data === "xboxAuth"}>
              <Trans key="auth:_trn_login.authenticating_xbox" />
            </Match>
            <Match when={enrollmentStatus.data === "mcLogin"}>
              <Trans key="auth:_trn_login.authenticating_minecraft" />
            </Match>
            <Match when={enrollmentStatus.data === "mcProfile"}>
              <Trans key="auth:_trn_login.retrieving_minecraft_profile" />
            </Match>
            <Match when={enrollmentStatus.data === "mcentitlements"}>
              <Trans key="auth:_trn_login.retrieving_minecraft_entitlements" />
            </Match>
            <Match when={true}>
              <Trans key="auth:_trn_login.authenticating" />
            </Match>
          </Switch>
        </span>
        <Progress />
      </div>

      {/* Retry button when expired - moved from footer */}
      <Show when={expired()}>
        <Button
          size="large"
          type="primary"
          fullWidth
          onClick={async () => {
            setLoading(true)
            try {
              await flow.startEnrollment(method())
            } catch (error) {
              console.error("Failed to retry enrollment:", error)
            } finally {
              setLoading(false)
            }
          }}
          loading={loading()}
          disabled={loading()}
        >
          <Trans key="auth:_trn_login.try_again" />
          <div class="i-hugeicons:refresh h-4 w-4" />
        </Button>
      </Show>

      {/* Alternative: Switch to device code (browser method only) */}
      <Show when={method() === "browser" && !expired()}>
        <div class="border-darkSlate-600 flex flex-col items-center gap-2 border-t pt-4">
          <p class="text-lightSlate-600 text-xs">
            <Trans key="auth:_trn_login.still_having_trouble" />
          </p>
          <button
            class="text-primary-400 hover:text-primary-300 cursor-pointer text-sm underline disabled:cursor-not-allowed disabled:opacity-50"
            onClick={handleSwitchToDeviceCode}
            disabled={loading()}
          >
            <Trans key="auth:_trn_login.try_device_code_instead" />
          </button>
        </div>
      </Show>
    </div>
  )
}
