import { Trans } from "@gd/i18n"
import { Button } from "@gd/ui"
import { Show } from "solid-js"

/**
 * Authentication complete step
 *
 * Shows success message and provides options to:
 * 1. Start using GDLauncher immediately (triggers welcome animation)
 * 2. Set up optional GDL account for cloud features
 * 3. Link existing GDL account (if found)
 */

interface CompleteStepProps {
  /** Whether user already has a GDL account linked */
  hasGDLAccount?: boolean
  /** Whether we found an existing GDL account in the cloud */
  foundExistingAccount?: boolean
  /** GDL account data if found */
  foundGDLAccountData?: {
    profileIconUrl: string
    nickname: string
    email: string
  } | null
  /** Callback to navigate to library (with animation) - saves "" if skipping */
  onContinue: () => void
  /** Callback to open GDL account setup modal */
  onSetupGDLAccount?: () => void
  /** Callback to link found existing account */
  onLinkExistingAccount?: () => void
}

export function CompleteStep(props: CompleteStepProps) {
  return (
    <div class="flex w-full flex-1 flex-col items-center justify-center gap-8 text-center">
      {/* Success icon */}
      <div class="bg-primary-500/10 flex h-24 w-24 items-center justify-center rounded-full">
        <div class="i-hugeicons:computer-phone-sync h-12 w-12 text-primary-400" />
      </div>

      {/* Success message */}
      <div class="flex flex-col gap-2">
        <h2 class="text-lightSlate-50 m-0 text-2xl font-bold">
          <Show
            when={props.hasGDLAccount}
            fallback={<Trans key="login.titles.authentication_complete" />}
          >
            <Trans key="login.titles.all_set" />
          </Show>
        </h2>
        <p class="text-lightSlate-600 m-0 text-sm">
          <Show
            when={props.hasGDLAccount}
            fallback={<Trans key="login.ready_to_launch" />}
          >
            <Trans key="login.cloud_sync_active" />
          </Show>
        </p>
      </div>

      {/* GDL Account section */}
      <Show when={props.foundExistingAccount}>
        {/* Found existing GDL account */}
        <div class="flex w-full max-w-96 flex-col gap-4">
          <div class="border-primary-500/40 bg-primary-500/5 relative w-full overflow-hidden rounded-lg border p-5">
            <div class="flex flex-col gap-4">
              <div class="flex items-start gap-3">
                <Show
                  when={props.foundGDLAccountData?.profileIconUrl}
                  fallback={
                    <div class="bg-primary-500/30 flex h-10 w-10 shrink-0 items-center justify-center rounded-lg">
                      <div class="i-hugeicons:user-account h-5 w-5 text-primary-400" />
                    </div>
                  }
                >
                  <img
                    src={props.foundGDLAccountData!.profileIconUrl}
                    alt="Profile"
                    class="h-10 w-10 shrink-0 rounded-lg object-cover"
                  />
                </Show>
                <div class="flex flex-col gap-1 text-left">
                  <h3 class="text-lightSlate-50 m-0 text-base font-bold">
                    <Trans
                      key="login.welcome_back_name"
                      options={{
                        name: props.foundGDLAccountData?.nickname || "User"
                      }}
                    />
                  </h3>
                  <p class="text-lightSlate-500 m-0 text-sm leading-relaxed">
                    <Trans key="login.sync_existing_account_description" />
                  </p>
                </div>
              </div>
              <ul class="text-lightSlate-500 m-0 flex list-none flex-col gap-2 pl-0 text-sm">
                <li class="flex items-start gap-2">
                  <div class="i-hugeicons:tick-02 h-4 w-4 text-primary-400 mt-0.5 shrink-0" />
                  <span>
                    <Trans key="login.benefit_shared_instances" />
                  </span>
                </li>
                <li class="flex items-start gap-2">
                  <div class="i-hugeicons:tick-02 h-4 w-4 text-primary-400 mt-0.5 shrink-0" />
                  <span>
                    <Trans key="login.benefit_metrics_sync" />
                  </span>
                </li>
                <li class="flex items-start gap-2">
                  <div class="i-hugeicons:tick-02 h-4 w-4 text-primary-400 mt-0.5 shrink-0" />
                  <span>
                    <Trans key="login.benefit_settings_sync" />
                  </span>
                </li>
              </ul>
              <Button
                size="large"
                variant="primary"
                fullWidth
                onClick={props.onLinkExistingAccount}
              >
                <Trans key="login.sync_account" />
                <div class="i-hugeicons:arrow-right-01 h-4 w-4 ml-2" />
              </Button>
            </div>
          </div>
          <p class="text-lightSlate-600 m-0 text-center text-xs">
            <Trans key="login.setup_later_in_settings" />
          </p>
        </div>
      </Show>

      <Show
        when={
          !props.hasGDLAccount &&
          !props.foundExistingAccount &&
          props.onSetupGDLAccount
        }
      >
        {/* No account - can create new one */}
        <div class="flex w-full max-w-96 flex-col gap-4">
          <div class="border-primary-500/40 bg-primary-500/5 relative w-full overflow-hidden rounded-lg border p-5">
            <div class="flex flex-col gap-4">
              <div class="flex items-start gap-3">
                <div class="bg-primary-500/30 flex h-10 w-10 shrink-0 items-center justify-center rounded-lg">
                  <div class="i-hugeicons:user-account h-5 w-5 text-primary-400" />
                </div>
                <div class="flex flex-col gap-1 text-left">
                  <h3 class="text-lightSlate-50 m-0 text-base font-bold">
                    <Trans key="login.enable_cloud_sync" />
                  </h3>
                  <p class="text-lightSlate-500 m-0 text-sm leading-relaxed">
                    <Trans key="login.unlock_features_description" />
                  </p>
                </div>
              </div>
              <ul class="text-lightSlate-500 m-0 flex list-none flex-col gap-2 pl-0 text-sm">
                <li class="flex items-start gap-2">
                  <div class="i-hugeicons:tick-02 h-4 w-4 text-primary-400 mt-0.5 shrink-0" />
                  <span>
                    <Trans key="login.benefit_share_with_friends" />
                  </span>
                </li>
                <li class="flex items-start gap-2">
                  <div class="i-hugeicons:tick-02 h-4 w-4 text-primary-400 mt-0.5 shrink-0" />
                  <span>
                    <Trans key="login.benefit_track_metrics" />
                  </span>
                </li>
                <li class="flex items-start gap-2">
                  <div class="i-hugeicons:tick-02 h-4 w-4 text-primary-400 mt-0.5 shrink-0" />
                  <span>
                    <Trans key="login.benefit_sync_devices" />
                  </span>
                </li>
              </ul>
              <p class="text-lightSlate-600 m-0 text-xs">
                <Trans key="login.quick_setup_time" />
              </p>
              <Button
                size="large"
                variant="primary"
                fullWidth
                onClick={props.onSetupGDLAccount}
              >
                <Trans key="login.enable_cloud_sync" />
                <div class="i-hugeicons:arrow-right-01 h-4 w-4 ml-2" />
              </Button>
            </div>
          </div>
          <p class="text-lightSlate-600 m-0 text-center text-xs">
            <Trans key="login.setup_later_in_settings" />
          </p>
        </div>
      </Show>
    </div>
  )
}
