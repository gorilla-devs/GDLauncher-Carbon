import { Show, createMemo } from "solid-js"
import { Trans } from "@gd/i18n"
import type { AuthStep } from "../flow/types"

/**
 * ProfileCreationStep
 *
 * Minecraft profile creation step for users without an existing profile.
 * Validates username format and checks availability before creation.
 * Shows real-time feedback and requirements.
 */

interface ProfileCreationStepProps {
  step: Extract<AuthStep, { type: "profile-creation" }>
  username: string
  onUsernameChange: (username: string) => void
  checking: boolean
  available: "available" | "taken" | "notallowed" | null
  error: string | null
}

export function ProfileCreationStep(props: ProfileCreationStepProps) {
  // Username validation
  const isValid = createMemo(() => {
    const name = props.username
    if (name.length === 0) return false
    if (name.length < 3 || name.length > 16) return false
    // Only letters, numbers, and underscores
    return /^[a-zA-Z0-9_]+$/.test(name)
  })

  return (
    <div class="flex w-full flex-1 flex-col items-center justify-center gap-8 p-6 text-center">
      {/* Content */}
      <div class="flex flex-col items-center justify-center gap-6">
        {/* Icon */}
        <div class="bg-primary-500/10 flex h-24 w-24 items-center justify-center rounded-full">
          <div class="i-hugeicons:user-add-02 h-12 w-12 text-primary-400" />
        </div>

        {/* Description */}
        <p class="text-lightSlate-400 m-0 max-w-md text-base leading-relaxed">
          <Trans key="auth:_trn_profile_creation.description" />
        </p>

        <div class="w-full max-w-md">
          <input
            type="text"
            value={props.username}
            onInput={(e) => props.onUsernameChange(e.currentTarget.value)}
            placeholder="Enter username"
            class="border-darkSlate-600 bg-darkSlate-700 text-lightSlate-50 placeholder:text-lightSlate-700 w-full rounded-lg border px-4 py-3 focus:border-primary-500 focus:outline-none"
            maxLength={16}
          />

          {/* Validation feedback */}
          <div class="mt-2 min-h-6 text-left text-sm">
            <Show when={props.username && !isValid()}>
              <p class="text-red-400">
                <Trans key="auth:_trn_profile_creation.invalid_format" />
              </p>
            </Show>
            <Show when={isValid() && props.checking}>
              <p class="text-lightSlate-500 flex items-center">
                <span class="i-hugeicons:loading-03 h-3 w-3 mr-2 inline animate-spin" />
                <Trans key="auth:_trn_profile_creation.checking" />
              </p>
            </Show>
            <Show
              when={
                isValid() && !props.checking && props.available === "available"
              }
            >
              <p class="text-green-400 flex items-center gap-2">
                <span class="i-hugeicons:checkmark-circle-02 h-4 w-4" />
                <Trans key="auth:_trn_profile_creation.available" />
              </p>
            </Show>
            <Show
              when={isValid() && !props.checking && props.available === "taken"}
            >
              <p class="text-red-400 flex items-center gap-2">
                <span class="i-hugeicons:cancel-circle h-4 w-4" />
                <Trans key="auth:_trn_profile_creation.taken" />
              </p>
            </Show>
            <Show
              when={
                isValid() && !props.checking && props.available === "notallowed"
              }
            >
              <p class="text-red-400 flex items-center gap-2">
                <span class="i-hugeicons:cancel-circle h-4 w-4" />
                <Trans key="auth:_trn_profile_creation.not_allowed" />
              </p>
            </Show>
          </div>

          {/* Requirements */}
          <div class="text-lightSlate-600 mt-4 text-left text-xs">
            <p class="mb-2 font-semibold">
              <Trans key="auth:_trn_profile_creation.requirements" />:
            </p>
            <ul class="list-disc space-y-1 pl-5">
              <li
                classList={{
                  "text-green-400":
                    props.username.length >= 3 && props.username.length <= 16
                }}
              >
                <Trans key="auth:_trn_profile_creation.requirement_length" />
              </li>
              <li
                classList={{
                  "text-green-400":
                    props.username.length > 0 &&
                    /^[a-zA-Z0-9_]*$/.test(props.username)
                }}
              >
                <Trans key="auth:_trn_profile_creation.requirement_characters" />
              </li>
              <li
                classList={{
                  "text-red-400": props.available === "notallowed"
                }}
              >
                <Trans key="auth:_trn_profile_creation.requirement_appropriate" />
              </li>
            </ul>
          </div>
        </div>

        <Show when={props.error}>
          <p class="text-red-400 text-sm">{props.error}</p>
        </Show>
      </div>
    </div>
  )
}
