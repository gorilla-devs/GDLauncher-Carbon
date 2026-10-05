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
  onCleanup,
  onMount
} from "solid-js"
import QRCode from "qrcode"
import { msToMinutes, msToSeconds, parseTwoDigitNumber } from "@/utils/helpers"
import GateAnimationRiveWrapper from "@/utils/GateAnimationRiveWrapper"
import GateAnimation from "../../../gate_animation.riv"
import { useFlow } from "../flow/FlowContext"
import {
  completeEnrollment,
  switchEnrollment
} from "../flow/enrollmentTransitions"
import { extractErrorDisplay } from "../flow/FlowController"
import type { AuthStep } from "../flow/types"
import { rspc } from "@/utils/rspcClient"
import { CancelledError, type Query } from "@tanstack/query-core"

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

// Shared with AuthFlow so branch lifetime can be exercised without starting
// the complete authentication page and its external dependencies.
export function EnrollingBranch(props: {
  step: EnrollingStepProps["step"] | null
}) {
  return (
    <Match when={props.step} keyed>
      {(step) => <EnrollingStep step={step} />}
    </Match>
  )
}

export function EnrollingStep(props: EnrollingStepProps) {
  const flow = useFlow()
  const [t] = useTransContext()
  const [loading, setLoading] = createSignal(false)
  const [ownedRevision, setOwnedRevision] = createSignal(
    flow.captureEnrollment()
  )
  const [statusReady, setStatusReady] = createSignal(false)
  const { client, queryClient } = rspc.useContext()
  const queryCache = queryClient.getQueryCache()
  let ownedQuery: Query | undefined
  let active = true
  // Keep the no-input RPC key and invalidation prefix. Cache ownership uses
  // Task19's existing operation revision, rather than a second generation.
  const statusHash = (revision: number) =>
    JSON.stringify(["account.enroll.getStatus", revision])
  const retireStatus = (revision: number) => {
    if (!flow.ownsEnrollment(revision)) return
    queryClient.removeQueries({
      predicate: (query) =>
        query === ownedQuery && query.queryHash === statusHash(revision)
    })
  }
  // A new observer must not admit a previous controller's cached status, even
  // when that controller happened to have the same numeric operation revision.
  queryClient.removeQueries({
    predicate: (query) => query.queryHash === statusHash(ownedRevision())
  })
  onMount(() => {
    ownedQuery = queryCache.get(statusHash(ownedRevision()))
    setStatusReady(true)
  })

  let qrCanvasRef: HTMLCanvasElement | undefined

  const method = () => props.step.method

  // Poll enrollment status from backend
  const enrollmentStatus = rspc.createQuery(() => {
    const revision = ownedRevision()
    return {
      queryKey: ["account.enroll.getStatus"],
      queryHash: statusHash(revision),
      enabled: statusReady() && !loading() && flow.ownsEnrollment(revision),
      queryFn: async ({ signal }) => {
        const requestQuery = ownedQuery
        const admitted = () =>
          active &&
          !signal.aborted &&
          statusReady() &&
          !loading() &&
          flow.ownsEnrollment(revision) &&
          !!requestQuery &&
          queryCache.get(statusHash(revision)) === requestQuery
        if (!admitted()) throw new CancelledError({ silent: true })
        try {
          const result = await client.query(["account.enroll.getStatus"], {
            signal
          })
          if (!admitted()) throw new CancelledError({ silent: true })
          return result
        } catch (error) {
          if (!admitted()) throw new CancelledError({ silent: true })
          throw error
        }
      },
      retry: false, // Terminal backend failures already have an explicit retry action.
      refetchInterval: 1000, // Poll every second
      refetchIntervalInBackground: true
    }
  })

  const ownsStatus = () =>
    active &&
    flow.ownsEnrollment(ownedRevision()) &&
    !!ownedQuery &&
    queryCache.get(statusHash(ownedRevision())) === ownedQuery
  const acceptsStatus = () => statusReady() && !loading() && ownsStatus()
  const status = () => (acceptsStatus() ? enrollmentStatus.data : undefined)
  const acceptedStatus = {
    get data() {
      return status()
    }
  }
  const [failure, setFailure] = createSignal<string>()

  createEffect(() => {
    const error = enrollmentStatus.error
    const data = status()
    if (!acceptsStatus()) return
    if (error) {
      setFailure(extractErrorDisplay(error))
    } else if (data && typeof data === "object" && "failed" in data) {
      setFailure(
        data.failed.description || t("errors:_trn_error.enrollment_failed")
      )
    } else if (
      !error &&
      data &&
      (typeof data !== "object" || !("failed" in data))
    ) {
      setFailure(undefined)
    }
  })

  const retryEnrollment = async () => {
    if (loading() || !ownsStatus()) return
    setLoading(true)
    setStatusReady(false)
    retireStatus(ownedRevision())
    setFailure(undefined)
    const started = flow.startEnrollment(method())
    const revision = flow.captureEnrollment()
    queryClient.removeQueries({
      predicate: (query) => query.queryHash === statusHash(revision)
    })
    ownedQuery = queryCache.build(queryClient, {
      queryKey: ["account.enroll.getStatus"] as readonly unknown[],
      queryHash: statusHash(revision)
    })
    setOwnedRevision(revision)
    try {
      await started
      if (!ownsStatus()) return
      // Begin has completed: discard any status published while it was pending,
      // then enable one fresh request for this operation's own cache entry.
      ownedQuery?.reset()
      setLoading(false)
      setStatusReady(true)
      await enrollmentStatus.refetch()
    } catch (error) {
      if (ownsStatus()) setFailure(extractErrorDisplay(error))
    } finally {
      if (ownsStatus()) setLoading(false)
    }
  }

  // Check if polling code data is available
  const hasPollingCode = () => {
    const s = status()
    return s && typeof s === "object" && "pollingCode" in s
  }

  // A missing deadline is different from a deadline whose remaining time is zero.
  const expiresAt = () => {
    const s = status()
    if (!s || typeof s === "string") return undefined
    const value =
      "pollingCode" in s
        ? s.pollingCode.expiresAt
        : "waitingForBrowser" in s
          ? s.waitingForBrowser.expires_at
          : undefined
    if (!value) return undefined
    const deadline = new Date(value).getTime()
    return Number.isFinite(deadline) ? deadline : undefined
  }
  const [countDown, setCountDown] = createSignal("0:00")
  const [expired, setExpired] = createSignal(false)

  const updateExpireTime = (now = Date.now(), deadline = expiresAt()) => {
    if (!acceptsStatus()) return
    const remaining = deadline === undefined ? 0 : Math.max(0, deadline - now)
    setCountDown(
      `${msToMinutes(remaining)}:${parseTwoDigitNumber(msToSeconds(remaining))}`
    )
    // Preserve the existing final subsecond early-expiry window, including
    // exact/past deadlines when a backgrounded timer skips that window.
    if (deadline !== undefined && deadline - now < 1000) setExpired(true)
    const s = status()
    if (
      s &&
      typeof s === "object" &&
      "failed" in s &&
      s.failed.errorType === "deviceCodeExpired"
    ) {
      setExpired(true)
    }
  }

  let previousDeadline: number | undefined
  let previousRevision = ownedRevision()
  createEffect(() => {
    const deadline = expiresAt()
    const revision = ownedRevision()
    const now = Date.now()
    if (
      (deadline !== previousDeadline || revision !== previousRevision) &&
      deadline !== undefined &&
      deadline > now
    ) {
      setExpired(false)
    }
    previousDeadline = deadline
    previousRevision = revision
    updateExpireTime(now, deadline)
  })

  // Set up countdown interval
  const interval = setInterval(() => {
    updateExpireTime()
  }, 1000)

  onCleanup(() => {
    active = false
    retireStatus(ownedRevision())
    clearInterval(interval)
  })

  // Handle switch to device code
  const handleSwitchToDeviceCode = () => {
    if (!loading() && ownsStatus()) {
      return switchEnrollment(flow, setLoading)
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
    const backendStatus = status()

    if (!backendStatus || failure() || !flow.ownsEnrollment(ownedRevision()))
      return

    // Handle successful completion
    if (typeof backendStatus === "object" && "complete" in backendStatus) {
      void completeEnrollment(flow, backendStatus.complete.flow_id)
      return
    }

    // Handle profile creation needed
    if (
      typeof backendStatus === "object" &&
      "needsProfileCreation" in backendStatus
    ) {
      flow.goToStep({
        type: "profile-creation",
        accessToken: backendStatus.needsProfileCreation.access_token,
        flowId: backendStatus.needsProfileCreation.flow_id,
        verificationRetry: backendStatus.needsProfileCreation.verification_retry
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

      <Show when={failure()}>
        <p role="alert" class="text-sm text-red-500">
          {failure()}
        </p>
      </Show>

      <Show when={!failure() && acceptsStatus()}>
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
                      const s = acceptedStatus.data
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
                        const s = acceptedStatus.data
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
                      const s = acceptedStatus.data
                      return s && typeof s === "object" && "pollingCode" in s
                        ? s.pollingCode.verificationUri
                        : ""
                    })()}
                    onCopy={() =>
                      toast.success(t("auth:_trn_login.link_copied"))
                    }
                  />
                </Show>
              </div>
            </Show>

            {/* Step 2: Enter the code */}
            <div
              data-testid="login-device-code"
              class="bg-darkSlate-800 flex flex-col items-center gap-2 rounded-xl px-6 py-4"
            >
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
                      const s = acceptedStatus.data
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
                      const s = acceptedStatus.data
                      return s && typeof s === "object" && "pollingCode" in s
                        ? s.pollingCode.userCode
                        : ""
                    })()}
                    onCopy={() =>
                      toast.success(
                        t("general:_trn_general_copied_to_clipboard")
                      )
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
              acceptedStatus.data &&
              typeof acceptedStatus.data === "string" &&
              !expired()
            ),
            "opacity-100": !!(
              acceptedStatus.data &&
              typeof acceptedStatus.data === "string" &&
              !expired()
            )
          }}
        >
          <span class="text-lightSlate-700 text-xs">
            <Switch>
              <Match
                when={
                  acceptedStatus.data &&
                  typeof acceptedStatus.data === "object" &&
                  "waitingForBrowser" in acceptedStatus.data
                }
              >
                <Trans key="auth:_trn_login.waiting_for_browser_confirmation" />
              </Match>
              <Match
                when={
                  acceptedStatus.data &&
                  typeof acceptedStatus.data === "object" &&
                  "pollingCode" in acceptedStatus.data
                }
              >
                <Trans key="auth:_trn_login.polling_microsoft_auth" />
              </Match>
              <Match when={acceptedStatus.data === "xboxAuth"}>
                <Trans key="auth:_trn_login.authenticating_xbox" />
              </Match>
              <Match when={acceptedStatus.data === "mcLogin"}>
                <Trans key="auth:_trn_login.authenticating_minecraft" />
              </Match>
              <Match when={acceptedStatus.data === "mcProfile"}>
                <Trans key="auth:_trn_login.retrieving_minecraft_profile" />
              </Match>
              <Match when={acceptedStatus.data === "mcentitlements"}>
                <Trans key="auth:_trn_login.retrieving_minecraft_entitlements" />
              </Match>
              <Match when={true}>
                <Trans key="auth:_trn_login.authenticating" />
              </Match>
            </Switch>
          </span>
          <Progress />
        </div>
      </Show>

      {/* Retry button when expired - moved from footer */}
      <Show when={expired() || failure()}>
        <Button
          size="large"
          type="primary"
          fullWidth
          onClick={retryEnrollment}
          loading={loading()}
          disabled={loading()}
        >
          <Trans key="auth:_trn_login.try_again" />
          <div class="i-hugeicons:refresh h-4 w-4" />
        </Button>
      </Show>

      {/* Alternative: Switch to device code (browser method only) */}
      <Show when={method() === "browser" && (!expired() || failure())}>
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
