/* eslint-disable i18next/no-literal-string */
import { Button, Dropdown, Input, Switch } from "@gd/ui";
import GDLauncherWideLogo from "/assets/images/gdlauncher_logo.svg";
import GDLauncherText from "/assets/images/GDLauncher_text.svg";
import { Trans, useTransContext } from "@gd/i18n";
import { queryClient, rspc } from "@/utils/rspcClient";
import SettingsData from "./settings.Privacy.data";
import { useRouteData } from "@solidjs/router";
import { createEffect } from "solid-js";
import { createStore } from "solid-js/store";
import { FESettings } from "@gd/core_module/bindings";

const Privacy = () => {
  const routeData: ReturnType<typeof SettingsData> = useRouteData();
  const [t, { changeLanguage }] = useTransContext();

  const [settings, setSettings] = createStore<FESettings>(
    // @ts-ignore
    routeData?.data?.data || {}
  );

  const settingsMutation = rspc.createMutation(["settings.setSettings"], {
    onMutate: (newSettings) => {
      if (newSettings.language) changeLanguage(newSettings.language as string);
      queryClient.setQueryData(["settings.getSettings"], newSettings);
    },
  });

  createEffect(() => {
    if (routeData.data.data) setSettings(routeData.data.data);
  });

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
    </div>
  );
};

export default Privacy;
