import { useModal } from ".."
import ModalLayout from "../ModalLayout"
import { Button, Input } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { createSignal } from "solid-js"
import { rspc } from "@/utils/rspcClient"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { Show } from "solid-js"
import { convertSecondsToHumanTime } from "@/utils/helpers"

const ChangeGDLAccountRecoveryEmail = () => {
  const [t] = useTransContext()
  const modalsContext = useModal()
  const [newRecoveryEmail, setNewRecoveryEmail] = createSignal("")
  const [isLoading, setIsLoading] = createSignal(false)
  const [cooldown, setCooldown] = createSignal(0)

  let cooldownInterval: ReturnType<typeof setInterval> | undefined

  const globalStore = useGlobalStore()

  const requestEmailChangeMutation = rspc.createMutation(() => ({
    mutationKey: ["account.requestEmailChange"]
  }))

  return (
    <ModalLayout
      title={t("accounts:_trn_change_recovery_email_title")}
      height="h-70"
      width="w-140"
    >
      <div class="flex h-full flex-col justify-between">
        <div class="flex flex-col gap-4">
          <div>
            <Trans key="accounts:_trn_change_recovery_email_description" />
          </div>
          <Input
            placeholder={t("placeholders:_trn_email_example")}
            value={newRecoveryEmail()}
            onInput={(e) => setNewRecoveryEmail(e.currentTarget.value)}
            disabled={!!cooldown()}
          />
          <Show when={cooldown()}>
            <div class="text-lightSlate-500 text-sm">
              <Trans
                key="auth:_trn_login.new_email_request_wait"
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
            disabled={isLoading() || !!cooldown()}
            onClick={async () => {
              const uuid = globalStore?.currentlySelectedAccountUuid?.data

              if (!uuid) {
                throw new Error("No active uuid")
              }

              const email = newRecoveryEmail()?.trim()

              if (!email) {
                throw new Error("No new recovery email")
              }

              setIsLoading(true)
              try {
                const result = await requestEmailChangeMutation.mutateAsync({
                  uuid,
                  email
                })

                if (result.status === "success") {
                  modalsContext?.closeModal()
                } else if (result.status === "failed" && result.value) {
                  setIsLoading(false)

                  setCooldown(result.value)
                  setNewRecoveryEmail(email)

                  cooldownInterval = setInterval(() => {
                    setCooldown((prev) => prev - 1)

                    if (cooldown() <= 0) {
                      setCooldown(0)
                      clearInterval(cooldownInterval)
                      cooldownInterval = undefined
                    }
                  }, 1000)
                }
              } catch (err) {
                console.error(err)
              }
              setIsLoading(false)
            }}
          >
            <Trans key="accounts:_trn_confirm" />
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default ChangeGDLAccountRecoveryEmail
