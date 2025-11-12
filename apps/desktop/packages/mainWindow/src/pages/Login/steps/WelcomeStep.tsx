import { Show } from "solid-js"
import { Trans } from "@gd/i18n"
import { PlaceholderGorilla } from "@/components/PlaceholderGorilla"
import { useFlow } from "../flow/FlowContext"

/**
 * WelcomeStep
 *
 * Simple welcome screen showing gorilla mascot and friendly greeting.
 * First step in the auth flow for new users.
 */
export function WelcomeStep() {
  const flow = useFlow()
  const hasActiveAccount = flow.data.accounts.length > 0

  return (
    <div class="flex w-full flex-1 flex-col items-center justify-center gap-8 p-6 text-center">
      {/* Content */}
      <div class="flex flex-col items-center justify-center gap-6">
        {/* Gorilla Mascot */}
        <PlaceholderGorilla
          size={12}
          variant="Waving Gorilla - Friendly Welcome"
        />

        {/* Welcome Message */}
        <p class="text-lightSlate-400 m-0 max-w-md text-base leading-relaxed">
          <Show
            when={hasActiveAccount}
            fallback={<Trans key="auth:_trn_login.welcome_tagline" />}
          >
            <Trans key="auth:_trn_login.welcome_returning_tagline" />
          </Show>
        </p>
      </div>
    </div>
  )
}
