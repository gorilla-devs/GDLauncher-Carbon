import { useGlobalStore } from "@/components/GlobalStoreContext"
import { convertSecondsToHumanTime } from "@/utils/helpers"
import { Trans, useTransContext } from "@gd/i18n"
import { Input, Tooltip, TooltipContent, TooltipTrigger } from "@gd/ui"
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
    <div class="flex w-full flex-1 flex-col items-center justify-between gap-5 p-10 text-center">
      <div class="flex w-full flex-col gap-4">
        <div class="flex items-center gap-2 text-lg">
          <Trans key="login.enter_your_recovery_email" />

          <Tooltip>
            <TooltipTrigger>
              <div class="i-ri:information-fill h-4 w-4" />
            </TooltipTrigger>
            <TooltipContent>
              <Trans key="login.recovery_email_description" />
            </TooltipContent>
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
          <div class="text-lightSlate-500 text-sm">
            <Trans
              key="login.new_email_request_wait"
              options={{
                time: convertSecondsToHumanTime(props.cooldown)
              }}
            />
          </div>
        </Show>

        <div class="flex items-center gap-2 text-lg">
          <Trans key="login.enter_your_nickname" />
          <Tooltip>
            <TooltipTrigger>
              <div class="i-ri:information-fill h-4 w-4" />
            </TooltipTrigger>
            <TooltipContent>
              <Trans key="login.nickname_description" />
            </TooltipContent>
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
