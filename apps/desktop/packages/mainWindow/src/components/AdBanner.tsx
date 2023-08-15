import adSize from "@/utils/adhelper";
import { Trans } from "@gd/i18n";
import { Match, Switch } from "solid-js";

export const AdsBanner = () => {
  const isSnapshotRelease = __APP_VERSION__.includes("-snapshot");
  const isBetaRelease = __APP_VERSION__.includes("-beta");
  const isAlphaRelease = __APP_VERSION__.includes("-alpha");

  return (
    <div
      style={{
        height: `${adSize.height}px`,
        width: `${adSize.width}px`,
      }}
    >
      <Switch>
        <Match when={isSnapshotRelease}>
          <div class="flex flex-col w-full box-border mb-8">
            <div class="flex w-full justify-center items-center h-10 font-bold box-border bg-red-900">
              <Trans key="adbanner.snapshot_title" />
            </div>
            <div class="w-full box-border flex-wrap p-4 border-1 border-lightSlate-600 border-x-solid border-b-solid">
              <Trans key="adbanner.snapshot_text" />
            </div>
          </div>
        </Match>
        <Match when={isBetaRelease}>
          <div class="flex flex-col w-full box-border mb-8">
            <div class="flex w-full justify-center items-center h-10 font-bold box-border bg-yellow-900">
              <Trans key="adbanner.beta_title" />
            </div>
            <div class="w-full box-border flex-wrap p-4 border-1 border-lightSlate-600 border-x-solid border-b-solid">
              <Trans key="adbanner.beta_text" />
            </div>
          </div>
        </Match>
        <Match when={isAlphaRelease}>
          <div class="flex flex-col w-full box-border mb-8">
            <div class="flex w-full justify-center items-center h-10 font-bold box-border bg-red-900">
              <Trans key="adbanner.alpha_title" />
            </div>
            <div class="w-full box-border flex-wrap p-4 border-1 border-lightSlate-600 border-x-solid border-b-solid">
              <Trans key="adbanner.alpha_text" />
            </div>
          </div>
        </Match>
      </Switch>
      <owadview class="relative z-[9999999]" />
    </div>
  );
};
