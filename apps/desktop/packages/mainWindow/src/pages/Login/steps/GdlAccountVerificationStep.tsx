import { Show, createSignal, onMount, onCleanup } from "solid-js"
import { Trans } from "@gd/i18n"
import { Button, toast } from "@gd/ui"
import type { AuthStep } from "../flow/types"
import { useFlow } from "../flow/FlowContext"
import { rspc, port } from "@/utils/rspcClient"
import { convertSecondsToHumanTime } from "@/utils/helpers"

interface GdlAccountVerificationStepProps {
  step: Extract<AuthStep, { type: "gdl-account-verification" }>
}

export function GdlAccountVerificationStep(
  props: GdlAccountVerificationStepProps
) {
  const flow = useFlow()

  const [isVerifying, setIsVerifying] = createSignal(true)
  const [resendCooldown, setResendCooldown] = createSignal(0)
  const [isResending, setIsResending] = createSignal(false)

  let abortController: AbortController | null = null
  let cooldownInterval: ReturnType<typeof setInterval> | undefined

  const requestNewVerificationTokenMutation = rspc.createMutation(() => ({
    mutationKey: ["account.requestNewVerificationToken"]
  }))

  const saveGdlAccountMutation = rspc.createMutation(() => ({
    mutationKey: ["account.saveGdlAccount"]
  }))

  const startVerificationPolling = async () => {
    if (abortController) {
      abortController.abort()
      abortController = null
    }

    abortController = new AbortController()
    setIsVerifying(true)

    try {
      const response = await fetch(
        `http://127.0.0.1:${port}/account/awaitForAccountVerification?uuid=${encodeURIComponent(props.step.uuid)}`,
        {
          signal: abortController.signal
        }
      )

      if (response.ok) {
        const result = await response.text()
        if (result === "ok") {
          await saveGdlAccountMutation.mutateAsync(props.step.uuid)
          toast.success("Email verified successfully!")
          await flow.exitFlow("library", flow.data.isFirstLaunch)
          return
        }
      }
      setIsVerifying(false)
      abortController = null
    } catch (error) {
      if ((error as Error).name === "AbortError") {
        abortController = null
        return
      }
      console.error("[GdlAccountVerificationStep] Polling error:", error)
      setIsVerifying(false)
      abortController = null
    }
  }

  const handleResend = async () => {
    if (resendCooldown() > 0 || isResending()) return

    setIsResending(true)

    try {
      if (cooldownInterval) {
        clearInterval(cooldownInterval)
        cooldownInterval = undefined
      }

      const result = await requestNewVerificationTokenMutation.mutateAsync(
        props.step.uuid
      )

      if (result.status === "success") {
        toast.success("Verification email sent")
        startVerificationPolling()
      } else if (result.status === "failed" && result.value) {
        setResendCooldown(result.value)
        cooldownInterval = setInterval(() => {
          setResendCooldown((prev) => {
            if (prev <= 1) {
              if (cooldownInterval) {
                clearInterval(cooldownInterval)
                cooldownInterval = undefined
              }
              return 0
            }
            return prev - 1
          })
        }, 1000)
      }
    } catch (error) {
      console.error("[GdlAccountVerificationStep] Resend error:", error)
      toast.error("Failed to resend verification email")
    } finally {
      setIsResending(false)
    }
  }

  onMount(() => {
    startVerificationPolling()
  })

  onCleanup(() => {
    if (abortController) {
      abortController.abort()
      abortController = null
    }
    if (cooldownInterval) {
      clearInterval(cooldownInterval)
      cooldownInterval = undefined
    }
  })

  return (
    <div class="flex w-full flex-1 flex-col items-center justify-center gap-8 p-6 text-center">
      <div class="bg-primary-500/10 flex h-24 w-24 items-center justify-center rounded-full">
        <div class="i-hugeicons:mail-receive-02 h-12 w-12 text-primary-400" />
      </div>

      <div class="flex flex-col gap-3">
        <h2 class="text-lightSlate-50 m-0 text-xl font-bold">
          <Trans key="auth:_trn_login.check_your_email_for_a_verification_link" />
        </h2>
        <p class="text-lightSlate-500 m-0 text-sm">
          <Trans
            key="auth:_trn_login.verification_sent_to"
            options={{ email: props.step.email }}
          />
        </p>
      </div>

      <Show when={isVerifying()}>
        <div class="flex items-center gap-3 text-lightSlate-500">
          <div class="i-svg-spinners:ring-resize h-5 w-5 text-primary-400" />
          <span class="text-sm">
            <Trans key="auth:_trn_login.waiting_for_verification" />
          </span>
        </div>
      </Show>

      <div class="flex flex-col gap-3 w-full max-w-xs">
        <Button
          size="medium"
          type="secondary"
          fullWidth
          onClick={handleResend}
          disabled={isResending() || resendCooldown() > 0}
          loading={isResending()}
        >
          <Show
            when={resendCooldown() > 0}
            fallback={<Trans key="auth:_trn_login.resend_email" />}
          >
            <Trans
              key="auth:_trn_login.resend_email_cooldown"
              options={{
                time: convertSecondsToHumanTime(resendCooldown())
              }}
            />
          </Show>
        </Button>

        <p class="text-lightSlate-600 m-0 text-xs">
          <Trans key="auth:_trn_login.check_spam_folder" />
        </p>
      </div>
    </div>
  )
}
