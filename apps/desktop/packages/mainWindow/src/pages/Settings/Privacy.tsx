/* eslint-disable i18next/no-literal-string */
import { Button } from "@gd/ui";
import { Trans } from "@gd/i18n";

const Privacy = () => {
  return (
    <div class="w-full flex flex-col py-5 box-border bg-darkSlate-800 h-auto px-6">
      <h2 class="m-0 mb-7 text-4">
        <Trans
          key="settings.Privacy"
          options={{
            defaultValue: "Privacy",
          }}
        />
      </h2>
      <div class="mb-6">
        <h5 class="mt-0 mb-2">
          <Trans key="settings.ads_personalization_title" />
        </h5>
        <div class="flex w-full justify-between">
          <p class="m-0 text-darkSlate-300 max-w-100">
            <Trans key="settings.ads_personalization_text" />
          </p>
          <Button
            type="secondary"
            size="small"
            rounded={false}
            onClick={() => {
              window?.openCMPWindow();
            }}
          >
            <Trans key="login.manage" />
          </Button>
        </div>
      </div>
      <div class="mb-6">
        <h5 class="mt-0 mb-2">
          <Trans key="settings.metrics_level_title" />
        </h5>
        <div class="flex w-full justify-between">
          <p class="m-0 text-darkSlate-300 max-w-100">
            <Trans key="settings.metrics_level_text" />
          </p>
          <Button
            type="secondary"
            size="small"
            rounded={false}
            onClick={() => {
              window?.openCMPWindow();
            }}
          >
            <Trans key="login.manage" />
          </Button>
        </div>
      </div>
    </div>
  );
};

export default Privacy;
