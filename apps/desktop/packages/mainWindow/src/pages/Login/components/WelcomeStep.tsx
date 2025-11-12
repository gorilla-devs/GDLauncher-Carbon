import { Show } from "solid-js"
import { Trans } from "@gd/i18n"
import { PlaceholderGorilla } from "@/components/PlaceholderGorilla"

/**
 * Simple welcome step - First impression
 *
 * Shows gorilla mascot and welcoming message
 * No forms, no checkboxes - just a friendly greeting
 */

interface WelcomeStepProps {
  /** Whether there's already an active account (returning user) */
  hasActiveAccount?: boolean
}

export function WelcomeStep(props: WelcomeStepProps) {
  return (
    <div class="flex w-full flex-1 flex-col items-center justify-center gap-8 p-6 text-center">
      {/* Gorilla Mascot */}
      <PlaceholderGorilla
        size={12}
        variant="Waving Gorilla - Friendly Welcome"
      />

      {/* Welcome Message */}
      <div class="flex flex-col items-center gap-3">
        <Show
          when={props.hasActiveAccount}
          fallback={
            <h2 class="text-lightSlate-50 m-0 text-3xl font-bold">
              <Trans key="auth:_trn_login.welcome_title" />
            </h2>
          }
        >
          <h2 class="text-lightSlate-50 m-0 text-3xl font-bold">
            <Trans key="auth:_trn_login.welcome_back" />
          </h2>
        </Show>

        <p class="text-lightSlate-400 max-w-md text-base leading-relaxed">
          <Show
            when={props.hasActiveAccount}
            fallback={<Trans key="auth:_trn_login.welcome_tagline" />}
          >
            <Trans key="auth:_trn_login.welcome_returning_tagline" />
          </Show>
        </p>
      </div>
    </div>
  )
}
