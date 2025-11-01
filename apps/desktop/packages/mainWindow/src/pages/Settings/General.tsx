import {
  Button,
  Select,
  SelectTrigger,
  SelectContent,
  SelectItem,
  SelectValue,
  Input,
  Switch
} from "@gd/ui"
import GDLauncherWideLogo from "/assets/images/gdlauncher_wide_logo_blue.svg"
import { Trans, useTransContext } from "@gd/i18n"
import { rspc } from "@/utils/rspcClient"
import SettingsData from "./settings.general.data"
import { useRouteData } from "@solidjs/router"
import { Show, createEffect } from "solid-js"
import { createStore } from "solid-js/store"
import {
  FELauncherActionOnGameLaunch,
  FEReleaseChannel,
  FESettings
} from "@gd/core_module/bindings"
import Row from "./components/Row"
import RightHandSide from "./components/RightHandSide"
import PageTitle from "./components/PageTitle"
import Title from "./components/Title"
import RowsContainer from "./components/RowsContainer"
import { useModal } from "@/managers/ModalsManager"

const General = () => {
  const routeData: ReturnType<typeof SettingsData> = useRouteData()
  const [t] = useTransContext()
  const modalsContext = useModal()

  const [settings, setSettings] = createStore<FESettings>(
    // @ts-ignore
    routeData?.data?.data || {}
  )

  const settingsMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setSettings"]
  }))

  createEffect(() => {
    if (routeData.data.data) setSettings(routeData.data.data)
  })

  const handleClearCache = () => {
    modalsContext?.openModal({
      name: "confirmCacheClear"
    })
  }

  const templateGameResolution = () => {
    return [
      "Standard:854x480",
      "Standard:1046x588",
      "Standard:1208x679",
      "Standard:1479x831"
    ]
  }

  const getResolutionLabel = (key: string) => {
    switch (key) {
      case "default":
        return "Default"
      case "Standard:854x480":
        return "854 x 480 (100%)"
      case "Standard:1046x588":
        return "1046 x 588 (150%)"
      case "Standard:1208x679":
        return "1208 x 679 (200%)"
      case "Standard:1479x831":
        return "1479 x 831 (300%)"
      case "custom":
        return "Custom"
      default:
        return key
    }
  }

  const gameResolutionDropdownKey = () => {
    if (!settings.gameResolution) return "default"

    if (settings.gameResolution.type === "Standard") {
      const gameResolution = settings.gameResolution.value.join("x")
      return `Standard:${gameResolution}`
    }

    return "custom"
  }

  return (
    <>
      <PageTitle>
        <Trans key="settings:General" />
      </PageTitle>
      <RowsContainer>
        <Row>
          <Title description={<Trans key="settings:release_channel_text" />}>
            <Trans key="settings:release_channel_title" />
          </Title>
          <RightHandSide>
            <Select
              value={settings.releaseChannel}
              onChange={(value) => {
                if (value) {
                  settingsMutation.mutate({
                    releaseChannel: {
                      Set: value as FEReleaseChannel
                    }
                  })
                }
              }}
              options={["stable", "beta", "alpha"]}
              itemComponent={(props) => (
                <SelectItem item={props.item}>
                  {props.item.rawValue === "stable" &&
                    t("settings:release_channel_stable")}
                  {props.item.rawValue === "beta" &&
                    t("settings:release_channel_beta")}
                  {props.item.rawValue === "alpha" &&
                    t("settings:release_channel_alpha")}
                </SelectItem>
              )}
            >
              <SelectTrigger>
                <SelectValue<string>>
                  {(state) => {
                    const val = state.selectedOption()
                    return val === "stable"
                      ? t("settings:release_channel_stable")
                      : val === "beta"
                        ? t("settings:release_channel_beta")
                        : val === "alpha"
                          ? t("settings:release_channel_alpha")
                          : ""
                  }}
                </SelectValue>
              </SelectTrigger>
              <SelectContent />
            </Select>
          </RightHandSide>
        </Row>
        <Row>
          <Title
            description={<Trans key="settings:concurrent_downloads_text" />}
          >
            <Trans key="settings:concurrent_downloads_title" />
          </Title>
          <RightHandSide>
            <Select
              value={(settings.concurrentDownloads || 1).toString()}
              onChange={(value) => {
                if (value) {
                  settingsMutation.mutate({
                    concurrentDownloads: {
                      Set: parseInt(value, 10)
                    }
                  })
                }
              }}
              options={Array.from({ length: 20 }, (_, i) => (i + 1).toString())}
              itemComponent={(props) => (
                <SelectItem item={props.item}>{props.item.rawValue}</SelectItem>
              )}
            >
              <SelectTrigger>
                <SelectValue<string>>
                  {(state) => state.selectedOption()}
                </SelectValue>
              </SelectTrigger>
              <SelectContent />
            </Select>
          </RightHandSide>
        </Row>
        <Row>
          <Title description={<Trans key="settings:game_resolution_text" />}>
            <Trans key="settings:game_resolution_title" />
          </Title>
          <RightHandSide>
            <div class="flex flex-col items-end gap-4">
              <Select
                value={gameResolutionDropdownKey()}
                placeholder={t("settings:resolution_presets")}
                onChange={(key) => {
                  if (!key) return

                  let value: {
                    type: "Standard" | "Custom"
                    value: [number, number]
                  } | null = null

                  if (key === "custom") {
                    value = {
                      type: "Custom",
                      value: [854, 480]
                    }
                  } else if (key === "default") {
                    value = null
                  } else {
                    const [width, height] = key
                      .toString()
                      .split(":")[1]
                      .split("x")
                    value = {
                      type: "Standard",
                      value: [parseInt(width, 10), parseInt(height, 10)]
                    }
                  }

                  settingsMutation.mutate({
                    gameResolution: {
                      Set: value
                    }
                  })
                }}
                options={["default", ...templateGameResolution(), "custom"]}
                itemComponent={(props) => (
                  <SelectItem item={props.item}>
                    {getResolutionLabel(props.item.rawValue)}
                  </SelectItem>
                )}
              >
                <SelectTrigger>
                  <SelectValue<string>>
                    {(state) =>
                      getResolutionLabel(state.selectedOption() || "")
                    }
                  </SelectValue>
                </SelectTrigger>
                <SelectContent />
              </Select>
              <Show when={settings.gameResolution?.type === "Custom"}>
                <div class="flex flex-col gap-4">
                  <div class="flex items-center justify-end gap-4">
                    <div>
                      <Trans key="settings:width" />
                    </div>
                    <Input
                      class="w-24"
                      type="number"
                      value={settings?.gameResolution?.value[0]}
                      onChange={(e) => {
                        settingsMutation.mutate({
                          gameResolution: {
                            Set: {
                              type: "Custom",
                              value: [
                                parseInt(e.currentTarget.value, 10),
                                settings?.gameResolution?.value[1]!
                              ]
                            }
                          }
                        })
                      }}
                    />
                  </div>
                  <div class="flex items-center justify-end gap-4">
                    <div>
                      <Trans key="settings:height" />
                    </div>
                    <Input
                      class="w-24"
                      type="number"
                      value={settings?.gameResolution?.value[1]}
                      onChange={(e) => {
                        settingsMutation.mutate({
                          gameResolution: {
                            Set: {
                              type: "Custom",
                              value: [
                                settings?.gameResolution?.value[0]!,
                                parseInt(e.currentTarget.value, 10)
                              ]
                            }
                          }
                        })
                      }}
                    />
                  </div>
                </div>
              </Show>
            </div>
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
                  discordIntegration: {
                    Set: e.currentTarget.checked
                  }
                })
              }}
            />
          </RightHandSide>
        </Row>
        <Row id="launcher_action_on_game_launch">
          <Title
            description={
              <Trans key="settings:launcher_action_on_game_launch_text" />
            }
          >
            <Trans key="settings:launcher_action_on_game_launch_title" />
          </Title>
          <RightHandSide>
            <Select
              value={settings.launcherActionOnGameLaunch}
              onChange={(value) => {
                if (!value) return

                const action = value as FELauncherActionOnGameLaunch

                settingsMutation.mutate({
                  launcherActionOnGameLaunch: {
                    Set: action
                  }
                })
              }}
              options={[
                "none",
                "minimizeWindow",
                "closeWindow",
                "hideWindow",
                "quitApp"
              ]}
              itemComponent={(props) => (
                <SelectItem item={props.item}>
                  {props.item.rawValue === "none" &&
                    t("settings:launcher_action_on_game_launch_none")}
                  {props.item.rawValue === "minimizeWindow" &&
                    t(
                      "settings:launcher_action_on_game_launch_minimize_window"
                    )}
                  {props.item.rawValue === "closeWindow" &&
                    t("settings:launcher_action_on_game_launch_close_window")}
                  {props.item.rawValue === "hideWindow" &&
                    t("settings:launcher_action_on_game_launch_hide_window")}
                  {props.item.rawValue === "quitApp" &&
                    t("settings:launcher_action_on_game_launch_quit_app")}
                </SelectItem>
              )}
            >
              <SelectTrigger>
                <SelectValue<string>>
                  {(state) => {
                    const val = state.selectedOption()
                    return val === "none"
                      ? t("settings:launcher_action_on_game_launch_none")
                      : val === "minimizeWindow"
                        ? t(
                            "settings:launcher_action_on_game_launch_minimize_window"
                          )
                        : val === "closeWindow"
                          ? t(
                              "settings:launcher_action_on_game_launch_close_window"
                            )
                          : val === "hideWindow"
                            ? t(
                                "settings:launcher_action_on_game_launch_hide_window"
                              )
                            : val === "quitApp"
                              ? t(
                                  "settings:launcher_action_on_game_launch_quit_app"
                                )
                              : ""
                  }}
                </SelectValue>
              </SelectTrigger>
              <SelectContent />
            </Select>
          </RightHandSide>
        </Row>
        <Row>
          <Title
            description={
              <Trans key="settings:show_window_close_warning_text" />
            }
          >
            <Trans key="settings:show_window_close_warning_title" />
          </Title>
          <RightHandSide>
            <Switch
              checked={settings.showAppCloseWarning}
              onChange={(e) => {
                settingsMutation.mutate({
                  showAppCloseWarning: {
                    Set: e.currentTarget.checked
                  }
                })
              }}
            />
          </RightHandSide>
        </Row>
        <Row>
          <Title
            description={
              <Trans key="settings:deletion_through_recycle_bin_text" />
            }
          >
            <Trans key="settings:deletion_through_recycle_bin_title" />
          </Title>
          <RightHandSide>
            <Switch
              checked={settings.deletionThroughRecycleBin}
              onChange={(e) => {
                settingsMutation.mutate({
                  deletionThroughRecycleBin: {
                    Set: e.currentTarget.checked
                  }
                })
              }}
            />
          </RightHandSide>
        </Row>
        <Row>
          <Title description={<Trans key="settings:potato_mode_text" />}>
            <Trans key="settings:potato_mode_title" />
          </Title>
          <RightHandSide>
            <Switch
              checked={settings.reducedMotion}
              onChange={(e) => {
                settingsMutation.mutate({
                  reducedMotion: {
                    Set: e.currentTarget.checked
                  }
                })
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
                modalsContext?.openModal({ name: "onBoarding" })
              }}
            >
              <div class="i-hugeicons:refresh text-lg" />
              <Trans key="settings:rerun_onboarding" />
            </Button>
          </RightHandSide>
        </Row>
        <Row class="bg-darkSlate-900 rounded-xl px-6 py-4">
          <img
            src={GDLauncherWideLogo}
            class="h-14 cursor-pointer transition-opacity duration-200 ease-in-out hover:opacity-80"
            onClick={() => {
              modalsContext?.openModal({
                name: "changelogs"
              })
            }}
          />
          <RightHandSide>
            <div>
              <div class="flex flex-col items-center justify-end gap-4 2xl:flex-row">
                <Button type="secondary" onClick={handleClearCache}>
                  <div class="flex items-center gap-2">
                    <div class="i-hugeicons:delete-02 h-5 w-5" />
                    <div>
                      <Trans key="settings:clear_cache_button" />
                    </div>
                  </div>
                </Button>
                <Button
                  type="secondary"
                  onClick={() => {
                    window.relaunch()
                  }}
                >
                  <div class="flex items-center gap-2">
                    <div class="i-hugeicons:arrow-reload-horizontal h-5 w-5" />
                    <div>
                      <Trans key="settings:restart_app" />
                    </div>
                  </div>
                </Button>
                <Button type="secondary">
                  <div class="flex items-center gap-2">
                    <div class="i-hugeicons:delete-02 h-5 w-5" />
                    <div>
                      <Trans key="settings:reset_all_data" />
                    </div>
                  </div>
                </Button>
              </div>
              <div class="text-darkSlate-300 text-4 mt-4">
                {"v"} {__APP_VERSION__}
              </div>
            </div>
          </RightHandSide>
        </Row>
      </RowsContainer>
    </>
  )
}

export default General
