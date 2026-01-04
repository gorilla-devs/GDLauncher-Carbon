import { useModal } from ".."
import ModalLayout from "../ModalLayout"
import { Button, Input } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { createEffect, createSignal, onCleanup, Show } from "solid-js"
import { queryClient, rspc } from "@/utils/rspcClient"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { convertSecondsToHumanTime } from "@/utils/helpers"

const ChangeGDLAccountNickname = () => {
  const [t] = useTransContext()
  const modalsContext = useModal()
  const [newNickname, setNewNickname] = createSignal("")
  const [isLoading, setIsLoading] = createSignal(false)
  const [error, setError] = createSignal<string | null>(null)
  const [cooldown, setCooldown] = createSignal(0)

  let cooldownInterval: ReturnType<typeof setInterval> | undefined

  const globalStore = useGlobalStore()

  const validGDLUser = () =>
    globalStore.gdlAccount.data?.status === "valid"
      ? globalStore.gdlAccount.data?.value
      : undefined

  // Initialize cooldown from GDL user data
  createEffect(() => {
    const timeout = validGDLUser()?.nicknameChangeTimeout
    if (timeout && timeout > 0) {
      setCooldown(timeout)
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

  const changeNicknameMutation = rspc.createMutation(() => ({
    mutationKey: ["account.changeGdlAccountNickname"]
  }))

  const isValid = () => {
    const nickname = newNickname().trim()
    return nickname.length >= 5 && nickname.length <= 20
  }

  return (
    <ModalLayout
      title={t("accounts:_trn_change_nickname_title")}
      height="h-70"
      width="w-140"
    >
      <div class="flex h-full flex-col justify-between">
        <div class="flex flex-col gap-4">
          <div>
            <Trans key="accounts:_trn_change_nickname_description" />
          </div>
          <Input
            placeholder={t("auth:_trn_login.nickname")}
            value={newNickname()}
            onInput={(e) => {
              setNewNickname(e.currentTarget.value)
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
                key="accounts:_trn_nickname_change_cooldown"
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

              const nickname = newNickname().trim()

              if (!nickname) {
                setError(t("auth:_trn_login.nickname_required"))
                return
              }

              if (nickname.length < 5) {
                setError(t("auth:_trn_login.nickname_too_short"))
                return
              }

              setIsLoading(true)
              try {
                const result = await changeNicknameMutation.mutateAsync({
                  uuid,
                  nickname
                })

                if (!result) {
                  // Mutation returned null - likely a backend error
                  setError(t("accounts:_trn_nickname_change_failed"))
                  setIsLoading(false)
                  return
                }

                if (result.status === "success") {
                  queryClient.invalidateQueries({
                    queryKey: ["account.getNicknameHistory"]
                  })
                  modalsContext?.closeModal()
                } else if (result.status === "failed" && result.value) {
                  setIsLoading(false)
                  setCooldown(result.value)
                  startCooldownTimer()
                } else if (result.status === "failed") {
                  setError(t("accounts:_trn_nickname_change_failed"))
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

export default ChangeGDLAccountNickname
