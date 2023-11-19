import { Button, Dropdown, Switch } from "@gd/ui";
import GDLauncherWideLogo from "/assets/images/gdlauncher_wide_logo_blue.svg";
import { Trans, useTransContext } from "@gd/i18n";
import { queryClient, rspc } from "@/utils/rspcClient";
import SettingsData from "./settings.general.data";
import { useRouteData } from "@solidjs/router";
import { createEffect } from "solid-js";
import { createStore } from "solid-js/store";
import { FEReleaseChannel, FESettings } from "@gd/core_module/bindings";
import Row from "./components/Row";
import RightHandSide from "./components/RightHandSide";
import PageTitle from "./components/PageTitle";
import Title from "./components/Title";
import RowsContainer from "./components/RowsContainer";
import { useModal } from "@/managers/ModalsManager";

const General = () => {
  const routeData: ReturnType<typeof SettingsData> = useRouteData();
  const [t] = useTransContext();
  const modalsContext = useModal();

  const [settings, setSettings] = createStore<FESettings>(
    // @ts-ignore
    routeData?.data?.data || {}
  );

  const settingsMutation = rspc.createMutation(["settings.setSettings"], {
    onMutate: (newSettings) => {
      queryClient.setQueryData(["settings.getSettings"], newSettings);
    }
  });

  createEffect(() => {
    if (routeData.data.data) setSettings(routeData.data.data);
  });

  return (
    <>
      <PageTitle>
        <Trans key="settings:General" />
      </PageTitle>
      <RowsContainer>
        <Row>
          <Title
            description={
              <Trans
                key="settings:release_channel_text"
                options={{
                  defaultValue: "Select the preferred release channel"
                }}
              />
            }
          >
            <Trans
              key="settings:release_channel_title"
              options={{
                defaultValue: "Release Channel"
              }}
            />
          </Title>
          <RightHandSide>
            <Dropdown
              value={settings.releaseChannel}
              options={[
                { label: t("settings:release_channel_stable"), key: "stable" },
                { label: t("settings:release_channel_beta"), key: "beta" },
                { label: t("settings:release_channel_alpha"), key: "alpha" }
              ]}
              onChange={(channel) => {
                settingsMutation.mutate({
                  releaseChannel: channel.key as FEReleaseChannel
                });
              }}
            />
          </RightHandSide>
        </Row>
        <Row>
          <Title
            description={
              <Trans
                key="settings:concurrent_downloads_text"
                options={{
                  defaultValue:
                    "Select the number of concurrent downloads. If you have slow connection, select at most 3"
                }}
              />
            }
          >
            <Trans
              key="settings:concurrent_downloads_title"
              options={{
                defaultValue: "Concurrent Downloads"
              }}
            />
          </Title>
          <RightHandSide>
            <Dropdown
              value={(settings.concurrentDownloads || "1").toString()}
              options={Array.from({ length: 20 }, (_, i) => ({
                label: (i + 1).toString(),
                key: (i + 1).toString()
              }))}
              onChange={(downloads) => {
                settingsMutation.mutate({
                  concurrentDownloads: parseInt(downloads.key as string, 10)
                });
              }}
            />
          </RightHandSide>
        </Row>
        <Row>
          <Title>
            <Trans
              key="settings:game_resolution_title"
              options={{
                defaultValue: "Game Resolution"
              }}
            />
          </Title>
          <RightHandSide>
            <Dropdown
              placeholder={t("settings:resolution_presets") || ""}
              options={[
                { label: "800x600", key: "800x600" },
                { label: "1024x768", key: "1024x768" },
                { label: "1920x1080", key: "1920x1080" }
              ]}
            />
          </RightHandSide>
        </Row>
        <Row>
          <Title
            description={
              <Trans
                key="settings:instance_sorting_text"
                options={{
                  defaultValue:
                    "Select the method in which instances should be sorted."
                }}
              />
            }
          >
            <Trans
              key="settings:instance_sorting_title"
              options={{
                defaultValue: "Instance Sorting"
              }}
            />
          </Title>
          <RightHandSide>
            <Dropdown
              options={[
                { label: "Alphabetical", key: "alphabetical" },
                { label: "Creation Date", key: "creation" }
              ]}
            />
          </RightHandSide>
        </Row>
        <Row>
          <Title
            description={
              <Trans
                key="settings:show_news_text"
                options={{
                  defaultValue: "Show or hide the news"
                }}
              />
            }
          >
            <Trans
              key="settings:show_news_title"
              options={{
                defaultValue: "Show news"
              }}
            />
          </Title>
          <RightHandSide>
            <Switch
              checked={settings.showNews}
              onChange={(e) => {
                settingsMutation.mutate({
                  showNews: e.currentTarget.checked
                });
              }}
            />
          </RightHandSide>
        </Row>
        <Row>
          <Title
            description={
              <Trans
                key="settings:discord_integration_text"
                options={{
                  defaultValue:
                    "Enable or disable discord integration. This display what are you playing in discord"
                }}
              />
            }
          >
            <Trans
              key="settings:discord_integration_title"
              options={{
                defaultValue: "Discord Integration"
              }}
            />
          </Title>
          <RightHandSide>
            <Switch
              checked={settings.discordIntegration}
              onChange={(e) => {
                settingsMutation.mutate({
                  discordIntegration: e.currentTarget.checked
                });
              }}
            />
          </RightHandSide>
        </Row>
        <Row>
          <Title
            description={
              <Trans
                key="settings:hide_launcher_playing_text"
                options={{
                  defaultValue:
                    "Automatically hide the launcher when launching an instance. You will still be able to open it from the icon tray."
                }}
              />
            }
          >
            <Trans
              key="settings:hide_launcher_playing_title"
              options={{
                defaultValue: "Hide launcher while playing"
              }}
            />
          </Title>
          <RightHandSide>
            <Switch />
          </RightHandSide>
        </Row>
        <Row>
          <Title
            description={
              <Trans
                key="settings:potato_mode_text"
                options={{
                  defaultValue:
                    "You got a potato PC? Don't worry! We got you covered. Enable this and all animations and special effects will be disabled."
                }}
              />
            }
          >
            <Trans
              key="settings:potato_mode_title"
              options={{
                defaultValue: "Potato PC mode"
              }}
            />
          </Title>
          <RightHandSide>
            <Switch
              checked={settings.reducedMotion}
              onChange={(e) => {
                settingsMutation.mutate({
                  reducedMotion: e.currentTarget.checked
                });
              }}
            />
          </RightHandSide>
        </Row>
        <Row>
          <Title>
            <Trans key="settings:rerun_onboarding" />
          </Title>
          <RightHandSide>
            <Button
              size="small"
              onClick={() => {
                modalsContext?.openModal({ name: "onBoarding" });
              }}
            >
              <Trans key="settings:rerun_onboarding" />
            </Button>
          </RightHandSide>
        </Row>
        <Row class="bg-darkSlate-900 rounded-xl px-6 py-4">
          <img src={GDLauncherWideLogo} class="h-14 cursor-pointer" />
          <RightHandSide>
            <div>
              <div class="flex justify-end gap-4 flex-col items-center 2xl:flex-row">
                <Button type="secondary">
                  <div class="flex items-center gap-2">
                    <i class="w-5 h-5 i-ri:restart-line" />
                    <div>
                      <Trans key="settings:restart_app" />
                    </div>
                  </div>
                </Button>
                <Button type="secondary">
                  <div class="flex items-center gap-2">
                    <i class="w-5 h-5 i-ri:delete-bin-7-line" />
                    <div>
                      <Trans key="settings:reset_all_data" />
                    </div>
                  </div>
                </Button>
              </div>
              <div class="text-darkSlate-300 mt-4 text-4">
                {"v"} {__APP_VERSION__}
              </div>
            </div>
          </RightHandSide>
        </Row>
      </RowsContainer>
    </>
  );
};

export default General;
