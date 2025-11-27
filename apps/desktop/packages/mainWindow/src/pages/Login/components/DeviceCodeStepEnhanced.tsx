/**
 * Enhanced Device Code Step with QR Code Support
 *
 * This component enhances the existing DeviceCodeStep with:
 * - QR code display (always visible alongside text code)
 * - Prominent copy button with success feedback
 * - Improved error handling using new backend error types
 * - Option to switch to browser method
 * - Better visual hierarchy and spacing
 */

import { Button, Progress, toast } from "@gd/ui"
import {
  createEffect,
  createSignal,
  Match,
  onCleanup,
  Show,
  Switch
} from "solid-js"
import { msToMinutes, msToSeconds, parseTwoDigitNumber } from "@/utils/helpers"
import { Setter } from "solid-js"
import { DeviceCode } from "@/components/CodeInput"
import { Trans, useTransContext } from "@gd/i18n"
import { getXboxErrorKey, getEnrollmentErrorKey } from "@gd/i18n/helpers"
import { rspc } from "@/utils/rspcClient"
import { DeviceCode as DeviceCodeType } from "@gd/core_module/bindings"
import { handleStatus } from "@/utils/login"
import { EnrollmentError } from "@gd/core_module/bindings"
import QRCode from "qrcode"
import type { CreateQueryResult } from "@tanstack/solid-query"

interface Props {
  deviceCodeObject: DeviceCodeType | null
  setDeviceCodeObject: Setter<DeviceCodeType | null>
  nextStep: () => void
  prevStep: () => void
  onSwitchToBrowser?: () => void
  enrollmentStatus: CreateQueryResult<any, any> | null
}

export function DeviceCodeStepEnhanced(props: Props) {
  const [error, setError] = createSignal<null | string>(null)

  let qrCanvasRef: HTMLCanvasElement | undefined

  const accountEnrollBeginMutation = rspc.createMutation(() => ({
    mutationKey: ["account.enroll.begin"],
    onError(error) {
      setError(error.message)
    }
  }))

  const userCode = () => props.deviceCodeObject?.userCode
  const oldUserCode = () => props.deviceCodeObject?.userCode
  const deviceCodeLink = () => props.deviceCodeObject?.verificationUri
  const expiresAt = () => props.deviceCodeObject?.expiresAt
  const expiresAtFormat = () => new Date(expiresAt() || "")?.getTime()
  const expiresAtMs = () => expiresAtFormat() - Date.now()

  const minutes = () => msToMinutes(expiresAtMs())
  const seconds = () => msToSeconds(expiresAtMs())
  const [countDown, setCountDown] = createSignal(
    `${minutes()}:${parseTwoDigitNumber(seconds())}`
  )
  const [expired, setExpired] = createSignal(false)
  const [t] = useTransContext()

  const resetCountDown = () => {
    setExpired(false)
    if (minutes() >= 0 && seconds() > 0) {
      setCountDown(`${minutes()}:${parseTwoDigitNumber(seconds())}`)
    }
  }

  const handleRefresh = async () => {
    resetCountDown()
    // Backend automatically cancels any existing enrollment
    accountEnrollBeginMutation.mutate(undefined)
  }

  const updateExpireTime = () => {
    if (minutes() <= 0 && seconds() <= 0) {
      setExpired(true)
    } else {
      resetCountDown()
    }
  }

  let interval: ReturnType<typeof setTimeout> | undefined

  createEffect(() => {
    if (interval !== undefined) {
      clearInterval(interval)
      interval = undefined
    }

    if (expired()) {
      // Just update the countdown display - enrollment will timeout naturally
      setCountDown(`${minutes()}:${parseTwoDigitNumber(seconds())}`)
    } else {
      interval = setInterval(() => {
        updateExpireTime()
      }, 1000)
    }
  })

  createEffect(() => {
    if (userCode() !== oldUserCode()) {
      resetCountDown()
    }
  })

  // Generate QR code when device code link changes
  createEffect(async () => {
    const link = deviceCodeLink()
    if (link && qrCanvasRef) {
      try {
        await QRCode.toCanvas(qrCanvasRef, link, {
          width: 200,
          margin: 2,
          color: {
            dark: "#000000", // Black for maximum contrast and scannability
            light: "#FFFFFF" // White background
          }
        })
      } catch (err) {
        console.error("Error generating QR code:", err)
        setError("Failed to generate QR code")
      }
    }
  })

  const handleErrorMessages = (error: EnrollmentError) => {
    const isCodeExpired = error.errorType === "deviceCodeExpired"

    if (isCodeExpired) {
      handleRefresh()
    } else if (error.errorType === "xboxAccount" && error.xboxError) {
      if (typeof error.xboxError === "string") {
        toast.error("Authentication Error", {
          description: t(getXboxErrorKey(error.xboxError as any))
        })
      } else {
        toast.error("Authentication Error", {
          description: `${t("errors:_trn_error.xbox_code")} ${error.xboxError.unknown}`
        })
      }
    } else {
      toast.error("Authentication Error", {
        description:
          error.description || t(getEnrollmentErrorKey(error.errorType as any))
      })
    }
  }

  createEffect(() => {
    if (props.enrollmentStatus) {
      handleStatus(props.enrollmentStatus, {
        onFail(error) {
          handleErrorMessages(error)
        }
      })
    }
  })

  onCleanup(() => {
    clearInterval(interval)
  })

  return (
    <div class="relative flex w-full flex-col items-center justify-center gap-6 text-center">
      {/* Device code display - Large and prominent */}
      <div class="flex flex-col items-center gap-2 w-full max-w-md">
        <DeviceCode
          expired={expired()}
          value={userCode() || ""}
          id="login-link-btn"
          handleRefresh={handleRefresh}
        />

        {/* Timer integrated below code */}
        <Show when={!expired()}>
          <p class="text-lightSlate-600 text-xs m-0">
            <Trans key="auth:_trn_login.expires_in" />{" "}
            <span class="text-lightSlate-400 font-medium">{countDown()}</span>
          </p>
        </Show>

        <Show when={expired()}>
          <p class="text-sm text-red-400 m-0">
            <Trans key="auth:_trn_login.code_expired_message" />
          </p>
        </Show>
      </div>

      {/* QR Code - Always visible */}
      <Show when={!expired()}>
        <div class="flex flex-col items-center gap-2">
          <div class="bg-white rounded-lg p-3">
            <canvas
              ref={qrCanvasRef}
              width="200"
              height="200"
              class="rounded"
            />
          </div>
          <p class="text-lightSlate-500 text-xs m-0">
            <Trans key="auth:_trn_login.scan_qr_code" />
          </p>
        </div>
      </Show>

      {/* Error display */}
      <Show when={error()}>
        <p class="text-red-400 m-0 text-sm">{error()}</p>
      </Show>

      {/* Primary action button */}
      <Show when={!expired()}>
        <div class="flex flex-col items-center gap-4 w-full max-w-md">
          <Button
            size="large"
            type="primary"
            onClick={() => {
              window.openExternalLink(deviceCodeLink() || "")
            }}
            disabled={!deviceCodeLink()}
            fullWidth
          >
            <div class="i-hugeicons:link-square-02 h-5 w-5" />
            <Trans key="auth:_trn_login.open_microsoft_login" />
          </Button>

          {/* Loading progress - Inline when active */}
          <Show when={props.enrollmentStatus?.data}>
            <div class="flex items-center gap-3 w-full">
              <Progress class="flex-1" />
              <span class="text-lightSlate-500 text-xs whitespace-nowrap">
                <Switch>
                  <Match when={props.enrollmentStatus?.data?.pollingCode}>
                    <Trans key="auth:_trn_login.polling_microsoft_auth" />
                  </Match>
                  <Match when={props.enrollmentStatus?.data === "xboxAuth"}>
                    <Trans key="auth:_trn_login.authenticating_xbox" />
                  </Match>
                  <Match when={props.enrollmentStatus?.data === "mcLogin"}>
                    <Trans key="auth:_trn_login.authenticating_minecraft" />
                  </Match>
                  <Match when={props.enrollmentStatus?.data === "mcProfile"}>
                    <Trans key="auth:_trn_login.retrieving_minecraft_profile" />
                  </Match>
                  <Match when={props.enrollmentStatus?.data?.mcEntitlements}>
                    <Trans key="auth:_trn_login.retrieving_minecraft_entitlements" />
                  </Match>
                </Switch>
              </span>
            </div>
          </Show>
        </div>
      </Show>

      {/* Alternative: Switch to browser auth */}
      <Show when={props.onSwitchToBrowser && !expired()}>
        <div class="border-darkSlate-600 flex flex-col items-center gap-2 border-t pt-4">
          <p class="text-lightSlate-600 text-xs m-0">
            <Trans key="auth:_trn_login.trouble_with_code" />
          </p>
          <button
            class="text-primary-400 hover:text-primary-300 cursor-pointer text-sm underline transition-colors"
            onClick={props.onSwitchToBrowser}
          >
            <Trans key="auth:_trn_login.try_browser_instead" />
          </button>
        </div>
      </Show>
    </div>
  )
}
