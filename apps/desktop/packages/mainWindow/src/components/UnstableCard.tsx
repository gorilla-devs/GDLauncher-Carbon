import { Trans } from "@gd/i18n"
import { Match, Show, Switch } from "solid-js"

const UnstableCard = () => {
  const isSnapshotRelease = __APP_VERSION__.includes("-snapshot") || false
  const isBetaRelease = __APP_VERSION__.includes("-beta")
  const isAlphaRelease = __APP_VERSION__.includes("-alpha")
  return (
    <Show when={!__SHOWCASE_MODE__}>
      <div>
        <Switch>
          <Match when={isSnapshotRelease}>
            <div class="mb-8 box-border flex w-full flex-col">
              <div class="box-border flex h-10 w-full items-center justify-center rounded-t-xl bg-red-900 font-bold">
                <Trans key="ads:_trn_adbanner.snapshot_title" />
              </div>
              <div class="border-1 border-darkSlate-600 border-x-solid border-b-solid box-border w-full flex-wrap rounded-b-xl p-4">
                <Trans key="ads:_trn_adbanner.snapshot_text" />
              </div>
            </div>
          </Match>
          <Match when={isBetaRelease}>
            <div class="mb-8 box-border flex w-full flex-col">
              <div class="box-border flex h-10 w-full items-center justify-center rounded-t-xl bg-yellow-900 font-bold">
                <Trans key="ads:_trn_adbanner.beta_title" />
              </div>
              <div class="border-1 border-darkSlate-600 border-x-solid border-b-solid box-border w-full flex-wrap rounded-b-xl p-4">
                <Trans key="ads:_trn_adbanner.beta_text" />
              </div>
            </div>
          </Match>
          <Match when={isAlphaRelease}>
            <div class="mb-8 box-border flex w-full flex-col">
              <div class="box-border flex h-10 w-full items-center justify-center rounded-t-xl bg-red-900 font-bold">
                <Trans key="ads:_trn_adbanner.alpha_title" />
              </div>
              <div class="border-1 border-darkSlate-600 border-x-solid border-b-solid box-border w-full flex-wrap rounded-b-xl p-4">
                <Trans key="ads:_trn_adbanner.alpha_text" />
              </div>
            </div>
          </Match>
        </Switch>
      </div>
    </Show>
  )
}

export default UnstableCard
