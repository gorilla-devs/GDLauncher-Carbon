import { Show, createMemo } from "solid-js"
import { Trans, useTransContext } from "@gd/i18n"
import { Tooltip, TooltipTrigger, TooltipContent } from "@gd/ui"
import type { AuthStep } from "../flow/types"

interface GdlAccountFormStepProps {
  step: Extract<AuthStep, { type: "gdl-account-form" }>
  email: string
  nickname: string
  onEmailChange: (email: string) => void
  onNicknameChange: (nickname: string) => void
  emailError?: string
  nicknameError?: string
}

export function GdlAccountFormStep(props: GdlAccountFormStepProps) {
  const [t] = useTransContext()

  const isEmailValid = createMemo(() => {
    const email = props.email.trim()
    if (email.length === 0) return false
    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/
    return emailRegex.test(email)
  })

  const isNicknameValid = createMemo(() => {
    const nickname = props.nickname.trim()
    return nickname.length >= 3
  })

  return (
    <div class="flex w-full flex-1 flex-col items-center justify-start gap-6 p-6 text-center overflow-y-auto">
      <div class="flex flex-col items-center gap-4">
        <div class="bg-primary-500/10 flex h-20 w-20 items-center justify-center rounded-full">
          <div class="i-hugeicons:user-account h-10 w-10 text-primary-400" />
        </div>

        <p class="text-lightSlate-400 m-0 max-w-md text-sm leading-relaxed">
          <Trans key="auth:_trn_login.enter_recovery_email_nickname" />
        </p>
      </div>

      <div class="w-full max-w-md flex flex-col gap-5">
        <div class="flex flex-col gap-2 text-left">
          <div class="flex items-center gap-2">
            <label class="text-lightSlate-400 text-sm font-medium">
              <Trans key="auth:_trn_login.recovery_email" />
            </label>
            <Tooltip>
              <TooltipTrigger>
                <div class="i-ri:information-fill w-4 h-4 text-lightSlate-600" />
              </TooltipTrigger>
              <TooltipContent>
                <Trans key="auth:_trn_login.recovery_email_description" />
              </TooltipContent>
            </Tooltip>
          </div>
          <input
            type="email"
            value={props.email}
            onInput={(e) => props.onEmailChange(e.currentTarget.value)}
            placeholder={t("placeholders:_trn_email_example")}
            class="border-darkSlate-600 bg-darkSlate-700 text-lightSlate-50 placeholder:text-lightSlate-700 w-full rounded-lg border px-4 py-3 focus:border-primary-500 focus:outline-none"
            classList={{
              "border-red-500": !!props.emailError
            }}
          />
          <Show when={props.emailError}>
            <p class="text-red-400 text-sm m-0">{props.emailError}</p>
          </Show>
          <Show when={!props.emailError && props.email && !isEmailValid()}>
            <p class="text-red-400 text-sm m-0">
              <Trans key="auth:_trn_login.email_invalid" />
            </p>
          </Show>
        </div>

        <div class="flex flex-col gap-2 text-left">
          <div class="flex items-center gap-2">
            <label class="text-lightSlate-400 text-sm font-medium">
              <Trans key="auth:_trn_login.nickname" />
            </label>
            <Tooltip>
              <TooltipTrigger>
                <div class="i-ri:information-fill w-4 h-4 text-lightSlate-600" />
              </TooltipTrigger>
              <TooltipContent>
                <Trans key="auth:_trn_login.nickname_description" />
              </TooltipContent>
            </Tooltip>
          </div>
          <input
            type="text"
            value={props.nickname}
            onInput={(e) => props.onNicknameChange(e.currentTarget.value)}
            placeholder={t("auth:_trn_login.nickname")}
            class="border-darkSlate-600 bg-darkSlate-700 text-lightSlate-50 placeholder:text-lightSlate-700 w-full rounded-lg border px-4 py-3 focus:border-primary-500 focus:outline-none"
            classList={{
              "border-red-500": !!props.nicknameError
            }}
          />
          <Show when={props.nicknameError}>
            <p class="text-red-400 text-sm m-0">{props.nicknameError}</p>
          </Show>
          <Show
            when={!props.nicknameError && props.nickname && !isNicknameValid()}
          >
            <p class="text-red-400 text-sm m-0">
              <Trans key="auth:_trn_login.nickname_too_short" />
            </p>
          </Show>
        </div>

        <p class="text-lightSlate-600 m-0 text-xs text-left">
          <Trans key="auth:_trn_login.verification_email_notice" />
        </p>
      </div>
    </div>
  )
}
