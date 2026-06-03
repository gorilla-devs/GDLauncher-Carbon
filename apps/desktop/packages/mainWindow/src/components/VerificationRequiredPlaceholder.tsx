import { Show, createSignal, onCleanup } from "solid-js"
import { Trans, useTransContext } from "@gd/i18n"
import { Button, toast } from "@gd/ui"
import { useGlobalStore } from "./GlobalStoreContext"
import { rspc } from "@/utils/rspcClient"
import { convertSecondsToHumanTime } from "@/utils/helpers"

/**
 * Inline placeholder shown when a feature requires email verification.
 * Drop this into any modal or page to replace content when unverified.
 *
 * Includes a "Resend verification email" button with cooldown handling.
 */
const VerificationRequiredPlaceholder = () => {
  const [t] = useTransContext()
  const globalStore = useGlobalStore()
  const [resendCooldown, setResendCooldown] = createSignal(0)
  const [isResending, setIsResending] = createSignal(false)

  let cooldownInterval: ReturnType<typeof setInterval> | undefined

  onCleanup(() => {
    if (cooldownInterval) {
      clearInterval(cooldownInterval)
      cooldownInterval = undefined
    }
  })

  const requestNewVerificationTokenMutation = rspc.createMutation(() => ({
    mutationKey: ["account.requestNewVerificationToken"]
  }))

  const activeUuid = () => globalStore.currentlySelectedAccountUuid.data

  const userEmail = () => {
    const data = globalStore.gdlAccount.data
    if (data?.status === "valid") return data.value.email
    return null
  }

  const handleResend = async () => {
    const uuid = activeUuid()
    if (!uuid || resendCooldown() > 0 || isResending()) return

    setIsResending(true)
    try {
      if (cooldownInterval) {
        clearInterval(cooldownInterval)
        cooldownInterval = undefined
      }

      const result = await requestNewVerificationTokenMutation.mutateAsync(uuid)

      if (result.status === "success") {
        toast.success(t("accounts:_trn_verification_email_sent"))
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
    } catch {
      toast.error(t("accounts:_trn_verification_email_failed"))
    } finally {
      setIsResending(false)
    }
  }

  return (
    <div class="flex flex-1 flex-col items-center justify-center gap-5 p-6 text-center">
      <div class="bg-yellow-500/10 flex h-16 w-16 items-center justify-center rounded-full">
        <div class="i-hugeicons:mail-receive-02 h-8 w-8 text-yellow-400" />
      </div>

      <div class="flex flex-col gap-2">
        <h3 class="text-lightSlate-50 m-0 text-base font-semibold">
          <Trans key="accounts:_trn_verification_required_title" />
        </h3>
        <p class="text-lightSlate-500 m-0 max-w-sm text-sm leading-relaxed">
          <Trans key="accounts:_trn_verification_required_description" />
        </p>
      </div>

      <Show when={userEmail()}>
        <p class="text-lightSlate-600 m-0 text-xs">
          <Trans
            key="accounts:_trn_verification_sent_to_email"
            options={{ email: userEmail()! }}
          />
        </p>
      </Show>

      <Button
        size="medium"
        type="secondary"
        onClick={handleResend}
        disabled={isResending() || resendCooldown() > 0}
        loading={isResending()}
      >
        <Show
          when={resendCooldown() > 0}
          fallback={<Trans key="accounts:_trn_resend_verification_email" />}
        >
          <Trans
            key="accounts:_trn_resend_verification_cooldown"
            options={{ time: convertSecondsToHumanTime(resendCooldown()) }}
          />
        </Show>
      </Button>
    </div>
  )
}

export default VerificationRequiredPlaceholder
