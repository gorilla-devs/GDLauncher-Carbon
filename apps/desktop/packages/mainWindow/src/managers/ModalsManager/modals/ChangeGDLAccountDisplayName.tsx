import { useModal } from ".."
import ModalLayout from "../ModalLayout"
import { Button, Input } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { createEffect, createSignal, onCleanup, Show } from "solid-js"
import { queryClient, rspc } from "@/utils/rspcClient"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { convertSecondsToHumanTime } from "@/utils/helpers"

const ChangeGDLAccountDisplayName = () => {
  const [t] = useTransContext()
  const modalsContext = useModal()
  const [newDisplayName, setNewDisplayName] = createSignal("")
  const [isLoading, setIsLoading] = createSignal(false)
  const [error, setError] = createSignal<string | null>(null)
  const [cooldown, setCooldown] = createSignal(0)

  let cooldownInterval: ReturnType<typeof setInterval> | undefined

  const globalStore = useGlobalStore()

  const validGDLUser = () =>
    globalStore.gdlAccount.data?.status === "valid"
      ? globalStore.gdlAccount.data?.value
      : undefined

  // Helper to calculate remaining seconds from an absolute UTC timestamp
  const getRemainingSeconds = (
    timeoutAt: string | null | undefined,
    fallbackSeconds: number | null | undefined
  ): number => {
    if (timeoutAt) {
      const expiresAt = new Date(timeoutAt).getTime()
      const remaining = Math.floor((expiresAt - Date.now()) / 1000)
      return Math.max(0, remaining)
    }
    return fallbackSeconds && fallbackSeconds > 0 ? fallbackSeconds : 0
  }

  // Initialize cooldown from GDL user data
  createEffect(() => {
    const user = validGDLUser()
    const remaining = getRemainingSeconds(
      user?.displayNameChangeTimeoutAt,
      user?.displayNameChangeTimeout
    )
    if (remaining > 0) {
      setCooldown(remaining)
      startCooldownTimer()
    }
  })

  const startCooldownTimer = () => {
    if (cooldownInterval) {
      clearInterval(cooldownInterval)
    }

    cooldownInterval = setInterval(() => {
      setCooldown((prev) => {
        if (prev <= 1) {
          clearInterval(cooldownInterval)
          cooldownInterval = undefined
          return 0
        }
        return prev - 1
      })
    }, 1000)
  }

  onCleanup(() => {
    if (cooldownInterval) {
      clearInterval(cooldownInterval)
    }
  })

  const changeDisplayNameMutation = rspc.createMutation(() => ({
    mutationKey: ["account.changeGdlAccountDisplayName"]
  }))

  const isValid = () => {
    const displayName = newDisplayName().trim()
    return displayName.length >= 5 && displayName.length <= 20
  }

  return (
    <ModalLayout
      title={t("accounts:_trn_change_display_name_title")}
      height="h-70"
      width="w-140"
    >
      <div class="flex h-full flex-col justify-between">
        <div class="flex flex-col gap-4">
          <div>
            <Trans key="accounts:_trn_change_display_name_description" />
          </div>
          <Input
            placeholder={t("auth:_trn_login.display_name")}
            value={newDisplayName()}
            onInput={(e) => {
              setNewDisplayName(e.currentTarget.value)
              setError(null)
            }}
            disabled={!!cooldown()}
          />
          <Show when={error()}>
            <div class="text-red-500 text-sm">{error()}</div>
          </Show>
          <Show when={cooldown()}>
            <div class="text-lightSlate-500 text-sm">
              <Trans
                key="accounts:_trn_display_name_change_cooldown"
                options={{
                  time: convertSecondsToHumanTime(cooldown())
                }}
              />
            </div>
          </Show>
        </div>

        <div class="flex w-full justify-between">
          <Button
            onClick={() => {
              modalsContext?.closeModal()
            }}
            type="secondary"
          >
            <Trans key="accounts:_trn_cancel" />
          </Button>
          <Button
            type="primary"
            disabled={isLoading() || !isValid() || !!cooldown()}
            onClick={async () => {
              const uuid = globalStore?.currentlySelectedAccountUuid?.data

              if (!uuid) {
                throw new Error("No active uuid")
              }

              const displayName = newDisplayName().trim()

              if (!displayName) {
                setError(t("auth:_trn_login.display_name_required"))
                return
              }

              if (displayName.length < 5) {
                setError(t("auth:_trn_login.display_name_too_short"))
                return
              }

              setIsLoading(true)
              try {
                const result = await changeDisplayNameMutation.mutateAsync({
                  uuid,
                  displayName: displayName
                })

                if (!result) {
                  // Mutation returned null - likely a backend error
                  setError(t("accounts:_trn_display_name_change_failed"))
                  setIsLoading(false)
                  return
                }

                if (result.status === "success") {
                  queryClient.invalidateQueries({
                    queryKey: ["account.getDisplayNameHistory"]
                  })
                  modalsContext?.closeModal()
                } else if (result.status === "failed" && result.value) {
                  setIsLoading(false)
                  setCooldown(result.value)
                  startCooldownTimer()
                } else if (result.status === "failed") {
                  setError(t("accounts:_trn_display_name_change_failed"))
                  setIsLoading(false)
                }
              } catch (err) {
                console.error(err)
                setError(String(err))
                setIsLoading(false)
              }
            }}
          >
            <Trans key="accounts:_trn_confirm" />
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default ChangeGDLAccountDisplayName
