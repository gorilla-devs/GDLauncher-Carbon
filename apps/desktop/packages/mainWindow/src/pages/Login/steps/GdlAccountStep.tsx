import { Trans, useTransContext } from "@gd/i18n"
import { Show } from "solid-js"
import { Collapsable } from "@gd/ui"
import { useFlow } from "../flow/FlowContext"
import type { AuthStep } from "../flow/types"

interface GdlAccountStepProps {
  step: Extract<AuthStep, { type: "gdl-account" }>
}

export function GdlAccountStep(props: GdlAccountStepProps) {
  const [t] = useTransContext()
  const flow = useFlow()

  const gdlState = () => props.step.gdlAccount || { type: "none" as const }

  return (
    <div class="flex w-full flex-1 flex-col items-center gap-6 p-6 text-center overflow-y-auto">
      <Show when={gdlState().type === "found-existing"}>
        {(() => {
          const state = gdlState()
          const data = state.type === "found-existing" ? state.data : null
          if (!data) return null

          return (
            <>
              <div class="bg-primary-500/10 flex h-20 w-20 items-center justify-center rounded-full">
                <div class="i-hugeicons:computer-phone-sync h-10 w-10 text-primary-400" />
              </div>

              <p class="text-lightSlate-400 m-0 max-w-md text-sm leading-relaxed">
                <Trans
                  key="auth:_trn_login.found_existing_account_description"
                  options={{ name: data.nickname || "User" }}
                />
              </p>

              <div class="w-full max-w-96 mt-2 flex flex-col gap-1">
                <h4 class="text-lightSlate-50 text-sm font-semibold mb-2 text-left">
                  <Trans key="auth:_trn_login.faqs" />
                </h4>
                <Collapsable
                  defaultOpened={false}
                  title={t("auth:_trn_login.what_is_a_gdlauncher_account")}
                  noPadding
                >
                  <p class="text-lightSlate-600 text-sm m-0 pt-2 pb-3 pl-6 text-left leading-relaxed">
                    <Trans key="auth:_trn_login.what_is_a_gdlauncher_account_text" />
                  </p>
                </Collapsable>
                <Collapsable
                  defaultOpened={false}
                  title={t("auth:_trn_login.how_does_it_work")}
                  noPadding
                >
                  <p class="text-lightSlate-600 text-sm m-0 pt-2 pb-3 pl-6 text-left leading-relaxed">
                    <Trans
                      key="auth:_trn_login.how_does_it_work_text"
                      options={{
                        account_id:
                          flow.data.accounts.find(
                            (acc) => acc.uuid === flow.data.activeUuid
                          )?.username || ""
                      }}
                    >
                      {""}
                      <span class="text-lightSlate-50 font-bold" />
                      {""}
                    </Trans>
                  </p>
                </Collapsable>
                <Collapsable
                  defaultOpened={false}
                  title={t(
                    "auth:_trn_login.what_if_i_lose_access_to_my_microsoft_account"
                  )}
                  noPadding
                >
                  <p class="text-lightSlate-600 text-sm m-0 pt-2 pb-3 pl-6 text-left leading-relaxed">
                    <Trans key="auth:_trn_login.what_if_i_lose_access_to_my_microsoft_account_text" />
                  </p>
                </Collapsable>
                <Collapsable
                  defaultOpened={false}
                  title={t(
                    "auth:_trn_login.what_happens_if_i_skip_the_account_creation"
                  )}
                  noPadding
                >
                  <p class="text-lightSlate-600 text-sm m-0 pt-2 pb-3 pl-6 text-left leading-relaxed">
                    <Trans key="auth:_trn_login.what_happens_if_i_skip_the_account_creation_text" />
                  </p>
                </Collapsable>
              </div>
            </>
          )
        })()}
      </Show>

      <Show when={gdlState().type === "none" && flow.data.gdlAccountId !== ""}>
        <>
          <div class="bg-primary-500/10 flex h-20 w-20 items-center justify-center rounded-full">
            <div class="i-hugeicons:cloud-upload h-10 w-10 text-primary-400" />
          </div>

          <p class="text-lightSlate-400 m-0 max-w-md text-sm leading-relaxed">
            <Trans key="auth:_trn_login.unlock_features_description" />
          </p>

          <div class="w-full max-w-96 mt-2 flex flex-col gap-1">
            <h4 class="text-lightSlate-50 text-sm font-semibold mb-2 text-left">
              <Trans key="auth:_trn_login.faqs" />
            </h4>
            <Collapsable
              defaultOpened={false}
              title={t("auth:_trn_login.what_is_a_gdlauncher_account")}
              noPadding
            >
              <p class="text-lightSlate-600 text-sm m-0 pt-2 pb-3 pl-6 text-left leading-relaxed">
                <Trans key="auth:_trn_login.what_is_a_gdlauncher_account_text" />
              </p>
            </Collapsable>
            <Collapsable
              defaultOpened={false}
              title={t("auth:_trn_login.how_does_it_work")}
              noPadding
            >
              <p class="text-lightSlate-600 text-sm m-0 pt-2 pb-3 pl-6 text-left leading-relaxed">
                <Trans
                  key="auth:_trn_login.how_does_it_work_text"
                  options={{
                    account_id:
                      flow.data.accounts.find(
                        (acc) => acc.uuid === flow.data.activeUuid
                      )?.username || ""
                  }}
                >
                  {""}
                  <span class="text-lightSlate-50 font-bold" />
                  {""}
                </Trans>
              </p>
            </Collapsable>
            <Collapsable
              defaultOpened={false}
              title={t(
                "auth:_trn_login.what_if_i_lose_access_to_my_microsoft_account"
              )}
              noPadding
            >
              <p class="text-lightSlate-600 text-sm m-0 pt-2 pb-3 pl-6 text-left leading-relaxed">
                <Trans key="auth:_trn_login.what_if_i_lose_access_to_my_microsoft_account_text" />
              </p>
            </Collapsable>
            <Collapsable
              defaultOpened={false}
              title={t(
                "auth:_trn_login.what_happens_if_i_skip_the_account_creation"
              )}
              noPadding
            >
              <p class="text-lightSlate-600 text-sm m-0 pt-2 pb-3 pl-6 text-left leading-relaxed">
                <Trans key="auth:_trn_login.what_happens_if_i_skip_the_account_creation_text" />
              </p>
            </Collapsable>
          </div>
        </>
      </Show>

      <Show
        when={
          gdlState().type === "linked" ||
          (gdlState().type === "none" && flow.data.gdlAccountId === "")
        }
      >
        <>
          <div class="bg-primary-500/10 flex h-24 w-24 items-center justify-center rounded-full">
            <div class="i-hugeicons:checkmark-circle-02 h-12 w-12 text-primary-400" />
          </div>

          <p class="text-lightSlate-400 m-0 max-w-md text-base leading-relaxed">
            <Show
              when={gdlState().type === "linked"}
              fallback={<Trans key="auth:_trn_login.ready_to_launch" />}
            >
              <Trans key="auth:_trn_login.cloud_sync_active" />
            </Show>
          </p>
        </>
      </Show>

      <Show when={gdlState().type === "error"}>
        {(() => {
          const state = gdlState()
          const errorMessage =
            state.type === "error" ? state.message : "Unknown error"

          return (
            <>
              <div class="bg-red-500/10 flex h-20 w-20 items-center justify-center rounded-full">
                <div class="i-hugeicons:alert-02 h-10 w-10 text-red-400" />
              </div>

              <div class="flex flex-col gap-2">
                <h3 class="text-lightSlate-50 m-0 text-lg font-bold">
                  <Trans key="auth:_trn_login.failed_to_check_account" />
                </h3>
                <p class="text-lightSlate-400 m-0 max-w-md text-sm leading-relaxed">
                  <Trans key="auth:_trn_login.failed_to_check_account_description" />
                </p>
                <p class="text-lightSlate-600 m-0 max-w-md text-xs leading-relaxed font-mono">
                  {errorMessage}
                </p>
              </div>
            </>
          )
        })()}
      </Show>
    </div>
  )
}
