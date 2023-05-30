/* eslint-disable i18next/no-literal-string */
import { Button, Dropdown, Input, Switch } from "@gd/ui";
import GDLauncherWideLogo from "/assets/images/gdlauncher_logo.svg";
import GDLauncherText from "/assets/images/GDLauncher_text.svg";
import { Trans, useTransContext } from "@gd/i18n";
import { queryClient, rspc } from "@/utils/rspcClient";
import SettingsData from "./settings.general.data";
import { useRouteData } from "@solidjs/router";
import { createEffect } from "solid-js";
import { createStore } from "solid-js/store";
import { FESettings } from "@gd/core_module/bindings";

const General = () => {
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
          key="settings.general"
          options={{
            defaultValue: "General",
          }}
        />
      </h2>
      <div class="mb-6">
        <h5 class="mt-0 mb-2">
          <Trans
            key="settings.choose_a_language_title"
            options={{
              defaultValue: "Language",
            }}
          />
        </h5>
        <div class="flex w-full justify-between">
          <p class="m-0 text-darkSlate-300 max-w-96">
            <Trans
              key="settings.choose_a_language_text"
              options={{
                defaultValue:
                  "Choose a language that is convenient for you and the launcher will be restarted",
              }}
            />
          </p>
          <Dropdown
            value={settings.language || "en"}
            options={[
              { label: t("languages.english"), key: "eng" },
              { label: t("languages.italian"), key: "it" },
            ]}
            onChange={(lang) => {
              settingsMutation.mutate({ language: lang.key as string });
            }}
          />
        </div>
      </div>
      <div class="mb-6">
        <h5 class="mt-0 mb-2">
          <Trans
            key="settings.release_channel_title"
            options={{
              defaultValue: "Release Channel",
            }}
          />
        </h5>
        <div class="flex w-full justify-between">
          <p class="text-darkSlate-300 m-0 max-w-96">
            <Trans
              key="settings.release_channel_text"
              options={{
                defaultValue: "Select the preferred release channel",
              }}
            />
          </p>
          <Dropdown
            value={settings.releaseChannel || "stable"}
            options={[
              { label: t("settings.release_channel_stable"), key: "stable" },
              { label: t("settings.release_channel_beta"), key: "beta" },
              { label: t("settings.release_channel_alpha"), key: "alpha" },
            ]}
            onChange={(channel) => {
              settingsMutation.mutate({
                releaseChannel: channel.key as string,
              });
            }}
          />
        </div>
      </div>
      <div class="mb-6">
        <h5 class="mt-0 mb-2">
          <Trans
            key="settings.concurrent_downloads_title"
            options={{
              defaultValue: "Concurrent Downloads",
            }}
          />
        </h5>
        <div class="flex w-full justify-between">
          <p class="text-darkSlate-300 m-0 max-w-96">
            <Trans
              key="settings.concurrent_downloads_text"
              options={{
                defaultValue:
                  "Select the number of concurrent downloads. If you have slow connection, select at most 3",
              }}
            />
          </p>
          <Dropdown
            value={(settings.concurrentDownloads || "1").toString()}
            options={Array.from({ length: 20 }, (_, i) => ({
              label: (i + 1).toString(),
              key: (i + 1).toString(),
            }))}
            onChange={(downloads) => {
              settingsMutation.mutate({
                concurrentDownloads: parseInt(downloads.key as string, 10),
              });
            }}
          />
        </div>
      </div>
      <div class="mb-6">
        <h5 class="mt-0 mb-2">
          <Trans
            key="settings.game_resolution_title"
            options={{
              defaultValue: "Game Resolution",
            }}
          />
        </h5>
        <div class="flex w-full justify-between">
          <div class="flex items-center gap-4">
            <Input
              class="w-20"
              placeholder={t("settings.resolution_width") || ""}
              value={"1024"}
            />
            x
            <Input
              class="w-20"
              placeholder={t("settings.resolution_height") || ""}
              value={"768"}
            />
          </div>
          <Dropdown
            value="en"
            placeholder={t("settings.resolution_presets") || ""}
            options={[
              { label: "800x600", key: "800x600" },
              { label: "1024x768", key: "1024x768" },
              { label: "1920x1080", key: "1920x1080" },
            ]}
          />
        </div>
      </div>
      <div class="mb-6">
        <h5 class="mt-0 mb-2">
          <Trans
            key="settings.instance_sorting_title"
            options={{
              defaultValue: "Instance Sorting",
            }}
          />
        </h5>
        <div class="flex w-full justify-between">
          <p class="text-darkSlate-300 max-w-96 m-0">
            <Trans
              key="settings.instance_sorting_text"
              options={{
                defaultValue:
                  "Select the method in which instances should be sorted.",
              }}
            />
          </p>
          <Dropdown
            value="en"
            options={[
              { label: "Alphabetical", key: "alphabetical" },
              { label: "creation", key: "creation" },
            ]}
          />
        </div>
      </div>
      {/* <div class="mb-6">
        <h5 class="mt-0 mb-2">
          <Trans
            key="expert_user_mod"
            options={{
              defaultValue: "Expert user mod",
            }}
          />
        </h5>
        <div class="flex w-full justify-between">
          <p class="text-darkSlate-300 max-w-96 m-0">
            <Trans
              key="settings_more_control"
              options={{
                defaultValue:
                  "Adds more control over the settings of your instances and java.",
              }}
            />
          </p>
          <Switch checked={true} />
        </div>
      </div> */}
      <div class="mb-6">
        <h5 class="mt-0 mb-2">
          <Trans
            key="settings.show_news_title"
            options={{
              defaultValue: "Show news",
            }}
          />
        </h5>
        <div class="flex w-full justify-between">
          <p class="text-darkSlate-300 max-w-96 m-0">
            <Trans
              key="settings.show_news_text"
              options={{
                defaultValue: "Show or hide the news",
              }}
            />
          </p>
          <Switch
            checked={settings.showNews}
            onChange={(e) => {
              settingsMutation.mutate({
                showNews: e.currentTarget.checked,
              });
            }}
          />
        </div>
      </div>
      <div class="mb-6">
        <h5 class="mt-0 mb-2">
          <Trans
            key="settings.discord_integration_title"
            options={{
              defaultValue: "Discord Integration",
            }}
          />
        </h5>
        <div class="flex w-full justify-between">
          <p class="text-darkSlate-300 max-w-96 m-0">
            <Trans
              key="settings.discord_integration_text"
              options={{
                defaultValue:
                  "Enable or disable discord integration. This display what are you playing in discord",
              }}
            />
          </p>
          <Switch
            checked={settings.discordIntegration}
            onChange={(e) => {
              settingsMutation.mutate({
                discordIntegration: e.currentTarget.checked,
              });
            }}
          />
        </div>
      </div>
      <div class="mb-6">
        <h5 class="mt-0 mb-2">
          <Trans
            key="settings.hide_launcher_playing_title"
            options={{
              defaultValue: "Hide launcher while playing",
            }}
          />
        </h5>
        <div class="flex w-full justify-between">
          <p class="text-darkSlate-300 max-w-96 m-0">
            <Trans
              key="settings.hide_launcher_playing_text"
              options={{
                defaultValue:
                  "Automatically hide the launcher when launching an instance. You will still be able to open it from the icon tray.",
              }}
            />
          </p>
          <Switch checked={false} />
        </div>
      </div>
      <div class="mb-6">
        <h5 class="mt-0 mb-2">
          <Trans
            key="settings.potato_mode_title"
            options={{
              defaultValue: "Potato PC mode",
            }}
          />
        </h5>
        <div class="flex w-full justify-between">
          <p class="text-darkSlate-300 max-w-96 m-0">
            <Trans
              key="settings.potato_mode_text"
              options={{
                defaultValue:
                  "You got a potato PC? Don't worry! We got you covered. Enable this and all animations and special effects will be disabled.",
              }}
            />
          </p>
          <Switch
            checked={settings.reducedMotion}
            onChange={(e) => {
              settingsMutation.mutate({
                reducedMotion: e.currentTarget.checked,
              });
            }}
          />
        </div>
      </div>
      <Button rounded={false} type="secondary" textColor="text-red-500">
        <Trans
          key="settings.reset"
          options={{
            defaultValue: "Reset all to default",
          }}
        />
      </Button>
      <div class="flex mb-6 mt-16 gap-24">
        <div class="flex gap-5">
          <img src={GDLauncherWideLogo} class="h-14 cursor-pointer" />
          <div class="flex flex-col">
            <img src={GDLauncherText} class="cursor-pointer h-5" />
            <p class="mb-0 mt-2">v.1.1.26</p>
          </div>
        </div>
        <p class="m-0 text-darkSlate-500">
          <Trans
            key="settings.last_version"
            options={{
              defaultValue:
                "You're currently on the latest version. We automatically check for updates and we will inform you whenever one is available.",
            }}
          />
        </p>
      </div>
    </div>
  );
};

export default General;
