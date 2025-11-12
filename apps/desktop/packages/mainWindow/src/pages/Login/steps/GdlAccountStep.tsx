import { Trans } from "@gd/i18n"
import { Show, createSignal } from "solid-js"
import { Button, toast } from "@gd/ui"
import { useFlow } from "../flow/FlowContext"
import type { AuthStep } from "../flow/types"

/**
 * GdlAccountStep
 *
 * GDL account setup/linking step showing authentication success.
 * Provides options to:
 * 1. Continue to library (triggers welcome animation if first launch)
 * 2. Set up optional GDL account for cloud features
 * 3. Link existing GDL account (if found)
 * All action buttons are in the content (not footer).
 */

interface GdlAccountStepProps {
  step: Extract<AuthStep, { type: "gdl-account" }>
}

export function GdlAccountStep(props: GdlAccountStepProps) {
  const flow = useFlow()
  const [loading, setLoading] = createSignal(false)

  const gdlState = () => props.step.gdlAccount || { type: "none" as const }

  const handleSetupGDLAccount = async () => {
    setLoading(true)
    try {
      await flow.setupGDLAccount()
      await flow.exitFlow("library", flow.data.isFirstLaunch)
    } catch (error) {
      console.error("[GdlAccountStep] Failed to setup GDL account:", error)
      toast.error("Failed to save GDL account preference", {
        description:
          error instanceof Error ? error.message : "Unknown error occurred"
      })
      setLoading(false)
    }
  }

  const handleLinkExistingAccount = async () => {
    const state = gdlState()
    if (state.type !== "found-existing") return

    setLoading(true)
    try {
      await flow.linkExistingGDLAccount(state.data)
      await flow.exitFlow("library", flow.data.isFirstLaunch)
    } catch (error) {
      console.error("[GdlAccountStep] Failed to link GDL account:", error)
      toast.error("Failed to link GDL account", {
        description:
          error instanceof Error ? error.message : "Unknown error occurred"
      })
      setLoading(false)
    }
  }

  const handleContinue = async () => {
    setLoading(true)
    try {
      await flow.exitFlow("library", flow.data.isFirstLaunch)
    } finally {
      setLoading(false)
    }
  }

  return (
    <div class="flex w-full flex-1 flex-col items-center justify-center gap-8 p-6 text-center">
      {/* Success icon */}
      <div class="bg-primary-500/10 flex h-24 w-24 items-center justify-center rounded-full">
        <div class="i-hugeicons:computer-phone-sync h-12 w-12 text-primary-400" />
      </div>

      {/* Success message */}
      <p class="text-lightSlate-400 m-0 max-w-md text-base leading-relaxed">
        <Show
          when={gdlState().type === "linked"}
          fallback={<Trans key="auth:_trn_login.ready_to_launch" />}
        >
          <Trans key="auth:_trn_login.cloud_sync_active" />
        </Show>
      </p>

      {/* Found existing GDL account */}
      <Show when={gdlState().type === "found-existing"}>
        {(() => {
          const state = gdlState()
          const data = state.type === "found-existing" ? state.data : null
          if (!data) return null

          return (
            <div class="flex w-full max-w-96 flex-col gap-4">
              <div class="border-primary-500/40 bg-primary-500/5 relative w-full overflow-hidden rounded-lg border p-5">
                <div class="flex flex-col gap-4">
                  <div class="flex items-start gap-3">
                    <Show
                      when={data.profileIconUrl}
                      fallback={
                        <div class="bg-primary-500/30 flex h-10 w-10 shrink-0 items-center justify-center rounded-lg">
                          <div class="i-hugeicons:user-account h-5 w-5 text-primary-400" />
                        </div>
                      }
                    >
                      <img
                        src={data.profileIconUrl}
                        alt="Profile"
                        class="h-10 w-10 shrink-0 rounded-lg object-cover"
                      />
                    </Show>
                    <div class="flex flex-col gap-1 text-left">
                      <h3 class="text-lightSlate-50 m-0 text-base font-bold">
                        <Trans
                          key="auth:_trn_login.welcome_back_name"
                          options={{
                            name: data.nickname || "User"
                          }}
                        />
                      </h3>
                      <p class="text-lightSlate-500 m-0 text-sm leading-relaxed">
                        <Trans key="auth:_trn_login.sync_existing_account_description" />
                      </p>
                    </div>
                  </div>
                  <ul class="text-lightSlate-500 m-0 flex list-none flex-col gap-2 pl-0 text-sm">
                    <li class="flex items-start gap-2">
                      <div class="i-hugeicons:tick-02 h-4 w-4 text-primary-400 mt-0.5 shrink-0" />
                      <span>
                        <Trans key="auth:_trn_login.benefit_shared_instances" />
                      </span>
                    </li>
                    <li class="flex items-start gap-2">
                      <div class="i-hugeicons:tick-02 h-4 w-4 text-primary-400 mt-0.5 shrink-0" />
                      <span>
                        <Trans key="auth:_trn_login.benefit_metrics_sync" />
                      </span>
                    </li>
                    <li class="flex items-start gap-2">
                      <div class="i-hugeicons:tick-02 h-4 w-4 text-primary-400 mt-0.5 shrink-0" />
                      <span>
                        <Trans key="auth:_trn_login.benefit_settings_sync" />
                      </span>
                    </li>
                  </ul>
                </div>
              </div>

              {/* Link Account button - moved from footer */}
              <Button
                size="large"
                variant="primary"
                fullWidth
                onClick={handleLinkExistingAccount}
                loading={loading()}
                disabled={loading()}
              >
                <Trans key="auth:_trn_login.sync_account" />
                <div class="i-hugeicons:arrow-right-01 h-4 w-4 ml-2" />
              </Button>

              <p class="text-lightSlate-600 m-0 text-center text-xs">
                <Trans key="auth:_trn_login.setup_later_in_settings" />
              </p>
            </div>
          )
        })()}
      </Show>

      {/* No GDL account - can create new one */}
      <Show when={gdlState().type === "none" && flow.data.gdlAccountId !== ""}>
        <div class="flex w-full max-w-96 flex-col gap-4">
          <div class="border-primary-500/40 bg-primary-500/5 relative w-full overflow-hidden rounded-lg border p-5">
            <div class="flex flex-col gap-4">
              <div class="flex items-start gap-3">
                <div class="bg-primary-500/30 flex h-10 w-10 shrink-0 items-center justify-center rounded-lg">
                  <div class="i-hugeicons:user-account h-5 w-5 text-primary-400" />
                </div>
                <div class="flex flex-col gap-1 text-left">
                  <h3 class="text-lightSlate-50 m-0 text-base font-bold">
                    <Trans key="auth:_trn_login.enable_cloud_sync" />
                  </h3>
                  <p class="text-lightSlate-500 m-0 text-sm leading-relaxed">
                    <Trans key="auth:_trn_login.unlock_features_description" />
                  </p>
                </div>
              </div>
              <ul class="text-lightSlate-500 m-0 flex list-none flex-col gap-2 pl-0 text-sm">
                <li class="flex items-start gap-2">
                  <div class="i-hugeicons:tick-02 h-4 w-4 text-primary-400 mt-0.5 shrink-0" />
                  <span>
                    <Trans key="auth:_trn_login.benefit_share_with_friends" />
                  </span>
                </li>
                <li class="flex items-start gap-2">
                  <div class="i-hugeicons:tick-02 h-4 w-4 text-primary-400 mt-0.5 shrink-0" />
                  <span>
                    <Trans key="auth:_trn_login.benefit_track_metrics" />
                  </span>
                </li>
                <li class="flex items-start gap-2">
                  <div class="i-hugeicons:tick-02 h-4 w-4 text-primary-400 mt-0.5 shrink-0" />
                  <span>
                    <Trans key="auth:_trn_login.benefit_sync_devices" />
                  </span>
                </li>
              </ul>
              <p class="text-lightSlate-600 m-0 text-xs">
                <Trans key="auth:_trn_login.quick_setup_time" />
              </p>
            </div>
          </div>

          {/* Enable Cloud Sync button - moved from footer */}
          <Button
            size="large"
            variant="primary"
            fullWidth
            onClick={handleSetupGDLAccount}
            loading={loading()}
            disabled={loading()}
          >
            <Trans key="auth:_trn_login.enable_cloud_sync" />
            <div class="i-hugeicons:arrow-right-01 h-4 w-4 ml-2" />
          </Button>

          <p class="text-lightSlate-600 m-0 text-center text-xs">
            <Trans key="auth:_trn_login.setup_later_in_settings" />
          </p>
        </div>
      </Show>

      {/* Already linked or user skipped - Continue button */}
      <Show
        when={
          gdlState().type === "linked" ||
          (gdlState().type === "none" && flow.data.gdlAccountId === "")
        }
      >
        <Button
          size="large"
          variant="primary"
          fullWidth
          class="max-w-96"
          onClick={handleContinue}
          loading={loading()}
          disabled={loading()}
        >
          <Trans key="general:_trn_continue" />
          <div class="i-hugeicons:arrow-right-01 h-4 w-4" />
        </Button>
      </Show>
    </div>
  )
}
