import { Trans } from "@gd/i18n"
import { Match, Show, Switch } from "solid-js"

const UnstableCard = () => {
  const isSnapshotRelease = __APP_VERSION__.includes("-snapshot") || false
  const isBetaRelease = __APP_VERSION__.includes("-beta")
  const isAlphaRelease = __APP_VERSION__.includes("-alpha")

  return (
    <Show when={!__SHOWCASE_MODE__}>
      <Switch>
        <Match when={isSnapshotRelease}>
          <div class="mb-4 flex w-full items-center justify-center gap-2 rounded-md bg-red-900 px-3 py-1 text-sm">
            <div class="i-hugeicons:alert-02 text-red-200" />
            <Trans key="ads:_trn_adbanner.snapshot_title" />
          </div>
        </Match>
        <Match when={isBetaRelease}>
          <div class="mb-4 flex w-full items-center justify-center gap-2 rounded-md bg-yellow-900 px-3 py-1 text-sm">
            <div class="i-hugeicons:alert-02 text-yellow-200" />
            <Trans key="ads:_trn_adbanner.beta_title" />
          </div>
        </Match>
        <Match when={isAlphaRelease}>
          <div class="mb-4 flex w-full items-center justify-center gap-2 rounded-md bg-red-900 px-3 py-1 text-sm">
            <div class="i-hugeicons:alert-02 text-red-200" />
            <Trans key="ads:_trn_adbanner.alpha_title" />
          </div>
        </Match>
      </Switch>
    </Show>
  )
}

export default UnstableCard
