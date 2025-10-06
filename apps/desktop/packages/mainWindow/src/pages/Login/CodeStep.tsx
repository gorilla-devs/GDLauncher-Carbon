import {
  Button,
  Progress,
  Popover,
  PopoverContent,
  PopoverTrigger
} from "@gd/ui"
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
import { toast } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { rspc } from "@/utils/rspcClient"
import { DeviceCodeObjectType } from "."
import GateAnimationRiveWrapper from "@/utils/GateAnimationRiveWrapper"
import GateAnimation from "../../gate_animation.riv"
import { handleStatus } from "@/utils/login"
import { useRouteData } from "@solidjs/router"
import fetchData from "./auth.login.data"
import { EnrollmentError } from "@gd/core_module/bindings"

interface Props {
  deviceCodeObject: DeviceCodeObjectType | null
  setDeviceCodeObject: Setter<DeviceCodeObjectType | null>
  nextStep: () => void
  prevStep: () => void
}

const CodeStep = (props: Props) => {
  const [error, setError] = createSignal<null | string>(null)

  const [shouldShowRetryMessage, setShouldShowRetryMessage] =
    createSignal(false)

  const accountEnrollCancelMutation = rspc.createMutation(() => ({
    mutationKey: ["account.enroll.cancel"],
    onError(error) {
      setError(error.message)
    }
  }))

  const accountEnrollBeginMutation = rspc.createMutation(() => ({
    mutationKey: ["account.enroll.begin"],
    onError(error) {
      setError(error.message)
    }
  }))

  const userCode = () => props.deviceCodeObject?.userCode
  const oldUserCode = () => props.deviceCodeObject?.userCode
  const deviceCodeLink = () => props.deviceCodeObject?.link
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
  const [loading, setLoading] = createSignal(false)

  const handleRefersh = async () => {
    resetCountDown()
    if (routeData.status.data) {
      accountEnrollCancelMutation.mutate(undefined)
    }
    accountEnrollBeginMutation.mutate(undefined)
  }

  const updateExpireTime = () => {
    if (minutes() <= 0 && seconds() <= 0) {
      setLoading(false)
      setExpired(true)
    } else {
      resetCountDown()
    }
  }

  let interval: ReturnType<typeof setTimeout> | undefined
  let retryMessageTimeoutId: number | undefined
  const routeData: ReturnType<typeof fetchData> = useRouteData()

  createEffect(() => {
    // Clear any existing interval before setting up a new one
    if (interval !== undefined) {
      clearInterval(interval)
      interval = undefined
    }

    if (expired()) {
      if (routeData.status.data) accountEnrollCancelMutation.mutate(undefined)
      setLoading(false)
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

  const handleErrorMessages = (error: EnrollmentError) => {
    const isCodeExpired = error === "deviceCodeExpired"

    if (isCodeExpired) {
      handleRefersh()
    } else if (typeof error === "string") {
      toast.error("Authentication Error", {
        description: t(`error.${error}`)
      })
    } else {
      if (typeof error.xboxAccount === "string")
        toast.error("Authentication Error", {
          description: t(`error.xbox_${error.xboxAccount}`)
        })
      else {
        toast.error("Authentication Error", {
          description: `${t("error.xbox_code")} ${error.xboxAccount.unknown}`
        })
      }
    }
  }

  createEffect(() => {
    handleStatus(routeData.status, {
      onFail(error) {
        handleErrorMessages(error)
      }
    })
  })

  onCleanup(() => {
    clearInterval(interval)
    if (retryMessageTimeoutId !== undefined) {
      clearTimeout(retryMessageTimeoutId)
    }
  })

  return (
    <div class="relative flex flex-col items-center justify-between text-center">
      <GateAnimationRiveWrapper width={80} height={80} src={GateAnimation} />
      <div class="absolute right-4 top-4">
        <Popover>
          <PopoverTrigger>
            <div class="text-lightSlate-700 hover:text-lightSlate-50 transition-color flex items-center text-sm duration-75">
              <div>
                <Trans key="login.need_help" />
              </div>
              <div class="i-ri:question-fill ml-2 h-4 w-4" />
            </div>
          </PopoverTrigger>
          <PopoverContent>
            <div class="max-w-100 px-4 pb-6 text-sm">
              <h3>
                <Trans key="login.troubles_logging_in" />
              </h3>
              <div class="pb-8 text-sm">
                <Trans key="login.link_not_working_help" />
              </div>
              <div
                class="text-lightSlate-600 hover:text-lightSlate-50 flex items-center gap-2"
                onClick={() => {
                  navigator.clipboard.writeText(deviceCodeLink()!)
                  toast.success("The link has been copied")
                }}
              >
                <div class="i-ri:link h-4 w-4" />
                <div>{deviceCodeLink()}</div>
              </div>
            </div>
          </PopoverContent>
        </Popover>
      </div>
      <div>
        <div class="flex flex-col items-center justify-center">
          <DeviceCode
            expired={expired()}
            value={userCode() || ""}
            id="login-link-btn"
            handleRefresh={handleRefersh}
          />
          <Show when={expired()}>
            <p class="text-sm text-red-500">
              <Trans key="login.code_expired_message" />
            </p>
          </Show>
        </div>
        <Show when={!expired()}>
          <p class="text-lightSlate-700 text-sm mt-2">
            <span class="text-lightSlate-500 mr-1">{countDown()}</span>
            <Trans key="login.before_expiring" />
          </p>
        </Show>
      </div>
      <Show when={error()}>
        <p class="m-0 text-red-500">{error()}</p>
      </Show>
      <div
        class="flex flex-col items-center justify-center"
        classList={{ "opacity-0": expired() }}
      >
        <p class="text-lightSlate-700 font-bold">
          <Trans key="login.enter_code_in_browser" />
        </p>
        <Button
          uppercase
          id="login-btn"
          class="mt-12 normal-case"
          onClick={() => {
            setLoading(true)
            navigator.clipboard.writeText(userCode() || "")
            window.openExternalLink(deviceCodeLink() || "")

            // Clear any existing timeout before setting a new one
            if (retryMessageTimeoutId !== undefined) {
              clearTimeout(retryMessageTimeoutId)
            }

            retryMessageTimeoutId = setTimeout(() => {
              setShouldShowRetryMessage(true)
              retryMessageTimeoutId = undefined
            }, 15 * 1000)
          }}
        >
          <Trans key="login.open_in_browser" />
          <div class="text-md i-ri:external-link-fill" />
        </Button>
        <p
          class="text-sm text-yellow-500"
          classList={{
            "opacity-0":
              !shouldShowRetryMessage() ||
              !(routeData.status.data as any)?.pollingCode
          }}
        >
          <Trans key="login.login_retry_message" />
        </p>
      </div>
      <div class="flex flex-col gap-2" classList={{ "opacity-0": !loading() }}>
        <span class="text-lightSlate-700 text-xs">
          <Switch>
            <Match when={(routeData.status.data as any)?.pollingCode}>
              <Trans key="login.polling_microsoft_auth" />
            </Match>
            <Match when={routeData.status.data === "xboxAuth"}>
              <Trans key="login.authenticating_xbox" />
            </Match>
            <Match when={routeData.status.data === "mcLogin"}>
              <Trans key="login.authenticating_minecraft" />
            </Match>
            <Match when={routeData.status.data === "mcProfile"}>
              <Trans key="login.retrieving_minecraft_profile" />
            </Match>
            <Match when={routeData.status.data === "mcentitlements"}>
              <Trans key="login.retrieving_minecraft_entitlements" />
            </Match>
          </Switch>
        </span>
        <Progress />
      </div>
    </div>
  )
}

export default CodeStep
