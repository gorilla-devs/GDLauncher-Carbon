import { Trans, useTransContext } from "@gd/i18n"
import { Button, Progress, toast } from "@gd/ui"
import {
  Show,
  Switch,
  Match,
  createSignal,
  createEffect,
  onCleanup
} from "solid-js"
import { msToMinutes, msToSeconds, parseTwoDigitNumber } from "@/utils/helpers"
import GateAnimationRiveWrapper from "@/utils/GateAnimationRiveWrapper"
import GateAnimation from "../../../gate_animation.riv"

/**
 * Browser OAuth authentication waiting screen
 *
 * Shows loading states and progress while waiting for browser authentication to complete.
 * Provides manual fallback options if auto-open fails.
 */

interface BrowserAuthStepProps {
  /** OAuth authorization URL */
  authUrl?: string
  /** Protocol redirect URI */
  redirectUri?: string
  /** When the auth session expires */
  expiresAt?: string
  /** Current enrollment stage */
  currentStage?: string
  /** Whether we're in an active enrollment */
  isEnrolling?: boolean
  /** Callback to switch to device code method */
  onSwitchToDeviceCode: () => void
  /** Callback to cancel enrollment */
  onCancel: () => void
  /** Callback to retry browser auth */
  onRetry: () => void
}

export function BrowserAuthStep(props: BrowserAuthStepProps) {
  const [t] = useTransContext()
  const [_browserOpened, setBrowserOpened] = createSignal(false)

  // Calculate time remaining
  const expiresAtMs = () => {
    if (!props.expiresAt) return 0
    return new Date(props.expiresAt).getTime() - Date.now()
  }

  const minutes = () => msToMinutes(expiresAtMs())
  const seconds = () => msToSeconds(expiresAtMs())
  const [countDown, setCountDown] = createSignal(
    `${minutes()}:${parseTwoDigitNumber(seconds())}`
  )
  const [expired, setExpired] = createSignal(false)

  // Reset expired state when props.expiresAt changes
  createEffect(() => {
    const expiresAt = props.expiresAt
    if (expiresAt) {
      const timeRemaining = new Date(expiresAt).getTime() - Date.now()
      // Reset expired if we have valid time remaining
      if (timeRemaining > 0) {
        setExpired(false)
      }
    }
  })

  // Update countdown timer
  const updateExpireTime = () => {
    // Only mark as expired if we have a valid expiresAt prop
    if (props.expiresAt && minutes() <= 0 && seconds() <= 0) {
      setExpired(true)
    } else {
      setCountDown(`${minutes()}:${parseTwoDigitNumber(seconds())}`)
    }
  }

  // Set up interval for countdown
  const interval = setInterval(() => {
    updateExpireTime()
  }, 1000)

  onCleanup(() => {
    clearInterval(interval)
  })

  // Backend opens the browser - we don't need to open it again
  // The backend already opens the browser in enroll.rs:197-201
  // if (props.authUrl && !browserOpened()) {
  //   window.openExternalLink(props.authUrl)
  //   setBrowserOpened(true)
  // }

  /**
   * Manually open browser again
   */
  const openBrowser = () => {
    if (props.authUrl) {
      window.openExternalLink(props.authUrl)
      setBrowserOpened(true)
      toast.success(t("login.browser_opened_toast"))
    }
  }

  return (
    <div class="relative flex w-full flex-1 flex-col items-center justify-center gap-6 text-center">
      {/* Animation */}
      <GateAnimationRiveWrapper width={80} height={80} src={GateAnimation} />

      {/* Expired state */}
      <Show when={expired()}>
        <div class="flex flex-col items-center gap-4">
          <p class="text-lightSlate-50 text-lg font-semibold">
            <Trans key="login.session_expired_title" />
          </p>
          <p class="text-lightSlate-700 text-sm">
            <Trans key="login.session_expired_message" />
          </p>
          <div class="flex gap-2">
            <Button size="medium" type="secondary" onClick={props.onCancel}>
              <Trans key="general.back" />
            </Button>
            <Button size="medium" onClick={props.onRetry}>
              <Trans key="login.try_again" />
            </Button>
          </div>
        </div>
      </Show>

      {/* Active waiting state */}
      <Show when={!expired()}>
        <div class="flex flex-col items-center gap-4">
          {/* Main message */}
          <div class="flex flex-col gap-2">
            <p class="text-lightSlate-50 text-lg font-semibold">
              <Trans key="login.waiting_for_browser_auth" />
            </p>
            <p class="text-lightSlate-700 text-sm">
              <Trans key="login.complete_signin_browser" />
            </p>
          </div>

          {/* Countdown */}
          <p class="text-lightSlate-700 text-sm">
            <span class="text-lightSlate-500 mr-1">{countDown()}</span>
            <Trans key="login.before_expiring" />
          </p>

          {/* Manual fallback options */}
          <div class="border-darkSlate-600 flex flex-col items-center gap-2 border-t pt-4">
            <p class="text-lightSlate-600 text-xs">
              <Trans key="login.browser_didnt_open" />
            </p>
            <Button size="small" type="secondary" onClick={openBrowser}>
              <div class="i-hugeicons:link-square-02" />
              <Trans key="login.open_browser_manually" />
            </Button>
          </div>
        </div>
      </Show>

      {/* Loading progress with stages */}
      <Show when={props.isEnrolling}>
        <div class="flex flex-col items-center gap-2">
          <span class="text-lightSlate-700 text-xs">
            <Switch>
              <Match when={props.currentStage === "waitingForBrowser"}>
                <Trans key="login.waiting_for_browser_confirmation" />
              </Match>
              <Match when={props.currentStage === "xboxAuth"}>
                <Trans key="login.authenticating_xbox" />
              </Match>
              <Match when={props.currentStage === "mcLogin"}>
                <Trans key="login.authenticating_minecraft" />
              </Match>
              <Match when={props.currentStage === "mcProfile"}>
                <Trans key="login.retrieving_minecraft_profile" />
              </Match>
              <Match when={props.currentStage === "mcEntitlements"}>
                <Trans key="login.retrieving_minecraft_entitlements" />
              </Match>
              <Match when={true}>
                <Trans key="login.authenticating" />
              </Match>
            </Switch>
          </span>
          <Progress />
        </div>
      </Show>

      {/* Alternative: Switch to device code */}
      <div class="border-darkSlate-600 flex flex-col items-center gap-2 border-t pt-4">
        <p class="text-lightSlate-600 text-xs">
          <Trans key="login.still_having_trouble" />
        </p>
        <button
          class="text-primary-400 hover:text-primary-300 cursor-pointer text-sm underline"
          onClick={props.onSwitchToDeviceCode}
        >
          <Trans key="login.try_device_code_instead" />
        </button>
      </div>
    </div>
  )
}
