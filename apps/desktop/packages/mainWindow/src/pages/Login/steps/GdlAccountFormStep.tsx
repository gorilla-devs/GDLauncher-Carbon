import { Show, createMemo } from "solid-js"
import { Trans, useTransContext } from "@gd/i18n"
import { Tooltip, TooltipTrigger, TooltipContent } from "@gd/ui"
import type { AuthStep } from "../flow/types"

interface GdlAccountFormStepProps {
  step: Extract<AuthStep, { type: "gdl-account-form" }>
  email: string
  displayName: string
  onEmailChange: (email: string) => void
  onDisplayNameChange: (displayName: string) => void
  emailError?: string
  displayNameError?: string
}

export function GdlAccountFormStep(props: GdlAccountFormStepProps) {
  const [t] = useTransContext()

  const isEmailValid = createMemo(() => {
    const email = props.email.trim()
    if (email.length === 0) return false
    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/
    return emailRegex.test(email)
  })

  const isDisplayNameValid = createMemo(() => {
    const displayName = props.displayName.trim()
    return displayName.length >= 3
  })

  return (
    <div class="flex w-full flex-1 flex-col items-center justify-start gap-6 p-6 text-center overflow-y-auto">
      <div class="flex flex-col items-center gap-4">
        <div class="bg-primary-500/10 flex h-20 w-20 items-center justify-center rounded-full">
          <div class="i-hugeicons:user-account h-10 w-10 text-primary-400" />
        </div>

        <p class="text-lightSlate-400 m-0 max-w-md text-sm leading-relaxed">
          <Trans key="auth:_trn_login.enter_recovery_email_display_name" />
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
              <Trans key="auth:_trn_login.display_name" />
            </label>
            <Tooltip>
              <TooltipTrigger>
                <div class="i-ri:information-fill w-4 h-4 text-lightSlate-600" />
              </TooltipTrigger>
              <TooltipContent>
                <Trans key="auth:_trn_login.display_name_description" />
              </TooltipContent>
            </Tooltip>
          </div>
          <input
            type="text"
            value={props.displayName}
            onInput={(e) => props.onDisplayNameChange(e.currentTarget.value)}
            placeholder={t("auth:_trn_login.display_name")}
            class="border-darkSlate-600 bg-darkSlate-700 text-lightSlate-50 placeholder:text-lightSlate-700 w-full rounded-lg border px-4 py-3 focus:border-primary-500 focus:outline-none"
            classList={{
              "border-red-500": !!props.displayNameError
            }}
          />
          <Show when={props.displayNameError}>
            <p class="text-red-400 text-sm m-0">{props.displayNameError}</p>
          </Show>
          <Show
            when={
              !props.displayNameError &&
              props.displayName &&
              !isDisplayNameValid()
            }
          >
            <p class="text-red-400 text-sm m-0">
              <Trans key="auth:_trn_login.display_name_too_short" />
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
