import { useGlobalStore } from "@/components/GlobalStoreContext"
import { convertSecondsToHumanTime } from "@/utils/helpers"
import { Trans, useTransContext } from "@gd/i18n"
import { Input, Tooltip } from "@gd/ui"
import { Show } from "solid-js"

interface Props {
  nextStep: () => void
  prevStep: () => void
  recoveryEmail: string | null
  setRecoveryEmail: (_: string | null) => void
  nickname: string | null
  setNickname: (_: string | null) => void
  cooldown: number
  acceptedHashedEmail: boolean
  setAcceptedHashedEmail: (_: (_: boolean) => boolean) => void
}

const GDLAccountCompletion = (props: Props) => {
  const [t] = useTransContext()

  const globalStore = useGlobalStore()

  const defaultNickname = () => {
    const account = globalStore.accounts.data?.find(
      (account) => account.uuid === globalStore.currentlySelectedAccount()?.uuid
    )

    if (!account) return ""

    return account.username
  }

  if (!props.nickname) {
    props.setNickname(defaultNickname())
  }

  return (
    <div class="flex-1 w-full flex flex-col justify-between items-center text-center gap-5 p-10">
      <div class="flex flex-col w-full gap-4">
        <div class="text-lg flex items-center gap-2">
          <Trans key="login.enter_your_recovery_email" />

          <Tooltip content={<Trans key="login.recovery_email_description" />}>
            <div class="i-ri:information-fill w-4 h-4" />
          </Tooltip>
        </div>
        <Input
          placeholder={t("login.recovery_email")}
          class="w-full"
          value={props.recoveryEmail || ""}
          disabled={!!props.cooldown}
          onSearch={(value) => {
            props.setRecoveryEmail(value)
          }}
        />
        <Show when={props.cooldown}>
          <div class="text-sm text-lightSlate-500">
            <Trans
              key="login.new_email_request_wait"
              options={{
                time: convertSecondsToHumanTime(props.cooldown)
              }}
            />
          </div>
        </Show>

        <div class="text-lg flex items-center gap-2">
          <Trans key="login.enter_your_nickname" />
          <Tooltip content={<Trans key="login.nickname_description" />}>
            <div class="i-ri:information-fill w-4 h-4" />
          </Tooltip>
        </div>
        <Input
          placeholder={t("login.nickname")}
          class="w-full"
          value={props.nickname ?? defaultNickname()}
          disabled={!!props.cooldown}
          onSearch={(value) => {
            props.setNickname(value)
          }}
        />

        {/* <div class="flex gap-2">
          <Checkbox
            checked={props.acceptedHashedEmail}
            onChange={() => {
              props.setAcceptedHashedEmail((prev) => !prev);
            }}
          />
          <p class="m-0 text-lightSlate-400 leading-5 text-xs select-none">
            <Trans key="login.enable_hashed_email">
              {""}
              <span
                class="cursor-pointer underline text-lightSlate-50"
                onClick={() => {
                  modalsContext?.openModal({
                    name: "privacyStatement"
                  });
                }}
              >
                {""}
              </span>
              {""}
            </Trans>
          </p>
        </div> */}
      </div>
    </div>
  )
}

export default GDLAccountCompletion
