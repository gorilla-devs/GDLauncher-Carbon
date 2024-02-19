import { generateSequence } from "@/utils/helpers";
import { port, queryClient, rspc } from "@/utils/rspcClient";
import { Trans, useTransContext } from "@gd/i18n";
import { Button, Dropdown, Input, Slider, Switch } from "@gd/ui";
import { useParams, useRouteData } from "@solidjs/router";
import fetchData from "../../instance.data";
import { Show, Suspense, createMemo } from "solid-js";
import { InstanceDetails } from "@gd/core_module/bindings";
import Title from "@/pages/Settings/components/Title";
import Row from "@/pages/Settings/components/Row";
import RowsContainer from "@/pages/Settings/components/RowsContainer";
import RightHandSide from "@/pages/Settings/components/RightHandSide";
import { setInstanceId } from "@/utils/browser";
import { useModal } from "@/managers/ModalsManager";

const Settings = () => {
  const [t] = useTransContext();
  const modalsContext = useModal();
  const params = useParams();
  const updateInstanceMutation = rspc.createMutation(
    ["instance.updateInstance"],
    {
      onMutate: (newData) => {
        queryClient.setQueryData(["instance.getInstanceDetails"], newData);
      }
    }
  );

  const routeData: ReturnType<typeof fetchData> = useRouteData();

  const initialJavaArgs = createMemo((prev: string | null) => {
    if (prev) return prev;

    return routeData?.instanceDetails?.data?.extraJavaArgs as string | null;
  }, null);

  const mbTotalRAM = () => Number(routeData.totalRam.data) / 1024 / 1024;

  const templateGameResolution = () => {
    return [
      { label: "854 x 480 (100%)", key: "Standard:854x480" },
      { label: "1046 x 588 (150%)", key: "Standard:1046x588" },
      { label: "1208 x 679 (200%)", key: "Standard:1208x679" },
      { label: "1479 x 831 (300%)", key: "Standard:1479x831" }
    ];
  };

  const gameResolutionDropdownKey = () => {
    if (routeData?.instanceDetails?.data?.gameResolution?.type === "Standard") {
      const gameResolution =
        routeData?.instanceDetails?.data?.gameResolution.value.join("x");
      return `Standard:${gameResolution}`;
    }

    return "custom";
  };

  return (
    <Suspense fallback={null}>
      <RowsContainer>
        <Show when={routeData?.instanceDetails?.data?.modpack}>
          <Row>
            <Title>
              <Trans key="instance_settings.modpack_info" />
            </Title>
          </Row>
          <div class="flex flex-col gap-4">
            <div class="flex items-center gap-4">
              <img
                class="h-13 w-13 rounded-lg"
                src={`http://127.0.0.1:${port}/instance/modpackIcon?instance_id=${params.id}`}
              />
              <div>
                <div class="text-lg font-bold">
                  {routeData.modpackInfo.data?.name}
                </div>
                <div>{routeData.modpackInfo.data?.version_name}</div>
              </div>
            </div>
            <div class="flex gap-4">
              <Show when={routeData.instanceDetails.data?.modpack?.locked}>
                <Button
                  type="outline"
                  onClick={() => {
                    updateInstanceMutation.mutate({
                      modpackLocked: {
                        Set: false
                      },
                      instance: parseInt(params.id, 10)
                    });
                  }}
                >
                  <i class="w-5 h-5 i-ri:lock-fill" />
                  <Trans key="instance_settings.unlock" />
                </Button>
              </Show>
              <Show when={!routeData.instanceDetails.data?.modpack?.locked}>
                <div class="flex items-center gap-2">
                  <i class="w-5 h-5 i-ri:lock-unlock-fill" />
                  <Trans key="instance_settings.unlocked" />
                </div>
              </Show>
              <Button
                type="outline"
                onClick={() => {
                  updateInstanceMutation.mutate({
                    modpackLocked: {
                      Set: null
                    },
                    instance: parseInt(params.id, 10)
                  });
                }}
              >
                <i class="w-5 h-5 i-ri:git-branch-fill" />
                <Trans key="instance_settings.unpair" />
              </Button>
              <Button
                type="outline"
                onClick={() => {
                  setInstanceId(parseInt(params.id, 10));
                  modalsContext?.openModal({
                    name: "modpack_version_update"
                  });
                }}
              >
                <i class="w-5 h-5 i-ri:arrow-left-right-fill" />
                <Trans key="instance_settings.update_version" />
              </Button>
            </div>
          </div>
        </Show>
        <Row>
          <Title>Instance Java Path / Profile</Title>
          <RightHandSide>
            <Switch
              checked={!!routeData?.instanceDetails?.data?.memory}
              onChange={(e) => {}}
            />
          </RightHandSide>
        </Row>
        <Show when={routeData?.instanceDetails?.data?.memory !== null}>
          <div class="flex justify-center px-2">
            <Slider
              min={0}
              max={mbTotalRAM()}
              steps={1000}
              value={routeData?.instanceDetails.data?.memory?.max_mb}
              marks={generateSequence(2048, mbTotalRAM())}
              onChange={(val) => {}}
              OnRelease={(val) => {}}
            />
          </div>
        </Show>
        <Row>
          <Title>
            <Trans key="instance_settings.java_memory_title" />
          </Title>
          <RightHandSide>
            <Switch
              checked={!!routeData?.instanceDetails?.data?.memory}
              onChange={(e) => {
                updateInstanceMutation.mutate({
                  memory: {
                    Set: e.target.checked
                      ? {
                          max_mb: Math.round(mbTotalRAM() / 2),
                          min_mb: Math.round(mbTotalRAM() / 2)
                        }
                      : null
                  },
                  instance: parseInt(params.id, 10)
                });
              }}
            />
          </RightHandSide>
        </Row>
        <Show when={routeData?.instanceDetails?.data?.memory !== null}>
          <div class="flex justify-center px-2">
            <Slider
              min={0}
              max={mbTotalRAM()}
              steps={1000}
              value={routeData?.instanceDetails.data?.memory?.max_mb}
              marks={generateSequence(2048, mbTotalRAM())}
              onChange={(val) => {
                if (
                  !val ||
                  routeData?.instanceDetails.data?.memory?.max_mb === val
                ) {
                  return;
                }
                queryClient.setQueryData(
                  ["instance.getInstanceDetails"],
                  (oldData: InstanceDetails | undefined) => {
                    if (!oldData) return;
                    oldData.memory = {
                      max_mb: val,
                      min_mb: val
                    };
                    return oldData;
                  }
                );
              }}
              OnRelease={(val) => {
                if (
                  !val ||
                  routeData?.instanceDetails.data?.memory?.max_mb === val
                ) {
                  return;
                }

                updateInstanceMutation.mutate({
                  memory: { Set: { max_mb: val, min_mb: val } },
                  instance: parseInt(params.id, 10)
                });
              }}
            />
          </div>
        </Show>

        <Row>
          <Title>
            <Trans key="instance_settings.java_arguments_title" />
          </Title>
          <Switch
            checked={
              routeData?.instanceDetails?.data?.extraJavaArgs !== null &&
              routeData?.instanceDetails?.data?.extraJavaArgs !== undefined
            }
            onChange={(e) => {
              const checked = e.target.checked;

              updateInstanceMutation.mutate({
                extraJavaArgs: { Set: checked ? "" : null },
                instance: parseInt(params.id, 10)
              });
            }}
          />
        </Row>
        <Show
          when={
            routeData?.instanceDetails?.data?.extraJavaArgs !== null &&
            routeData?.instanceDetails?.data?.extraJavaArgs !== undefined
          }
        >
          <div class="flex w-full justify-between items-center -mt-8">
            <h5 class="text-lightSlate-800">
              <Trans key="instance_settings.prepend_global_java_args" />
            </h5>
            <Switch
              checked={routeData?.instanceDetails?.data?.globalJavaArgs}
              onChange={(e) => {
                const checked = e.target.checked;

                updateInstanceMutation.mutate({
                  globalJavaArgs: { Set: checked },
                  instance: parseInt(params.id, 10)
                });
              }}
            />
          </div>
          <div class="flex w-full gap-4 items-center">
            <Show when={routeData?.instanceDetails?.data?.globalJavaArgs}>
              {"{GLOBAL_JAVA_ARGS}"}
              <div>+</div>
            </Show>
            <Input
              class="w-full"
              value={routeData?.instanceDetails?.data?.extraJavaArgs || ""}
              onChange={(e) => {
                updateInstanceMutation.mutate({
                  extraJavaArgs: { Set: e.target.value },
                  instance: parseInt(params.id, 10)
                });
              }}
            />
            <Button
              rounded={false}
              type="secondary"
              class="h-10"
              textColor="text-red-500"
              onClick={() => {
                updateInstanceMutation.mutate({
                  extraJavaArgs: { Set: initialJavaArgs() },
                  instance: parseInt(params.id, 10)
                });
              }}
            >
              <i class="w-5 h-5 i-ri:arrow-go-back-fill" />
            </Button>
            <Button
              rounded={false}
              type="secondary"
              class="h-10"
              textColor="text-red-500"
              onClick={() => {
                updateInstanceMutation.mutate({
                  extraJavaArgs: { Set: "" },
                  instance: parseInt(params.id, 10)
                });
              }}
            >
              <i class="w-5 h-5 i-ri:close-fill" />
            </Button>
          </div>
        </Show>
        <Row>
          <Title
            description={<Trans key="instance_settings.game_resolution_text" />}
          >
            <Trans key="instance_settings.game_resolution_title" />
          </Title>
          <RightHandSide>
            <Switch
              checked={!!routeData?.instanceDetails?.data?.gameResolution}
              onChange={(e) => {
                updateInstanceMutation.mutate({
                  gameResolution: {
                    Set: e.target.checked
                      ? { type: "Standard", value: [854, 480] }
                      : null
                  },
                  instance: parseInt(params.id, 10)
                });
              }}
            />
          </RightHandSide>
        </Row>
        <Show when={routeData?.instanceDetails?.data?.gameResolution}>
          <div class="flex gap-4">
            <Dropdown
              value={gameResolutionDropdownKey()}
              placeholder={t("settings:resolution_presets")}
              options={[
                ...templateGameResolution(),
                { label: "Custom", key: "custom" }
              ]}
              onChange={(option) => {
                let value: {
                  type: "Standard" | "Custom";
                  value: [number, number];
                } | null = null;

                if (option.key === "custom") {
                  value = {
                    type: "Custom",
                    value: [854, 480]
                  };
                } else {
                  const [width, height] = option.key
                    .toString()
                    .split(":")[1]
                    .split("x");
                  value = {
                    type: "Standard",
                    value: [parseInt(width, 10), parseInt(height, 10)]
                  };
                }

                updateInstanceMutation.mutate({
                  gameResolution: {
                    Set: value
                  },
                  instance: parseInt(params.id, 10)
                });
              }}
            />
            <Show
              when={
                routeData?.instanceDetails?.data?.gameResolution?.type ===
                "Custom"
              }
            >
              <div class="flex gap-4">
                <div class="flex items-center gap-4">
                  <div>
                    <Trans key="instance_settings.width" />
                  </div>
                  <Input
                    class="w-24"
                    type="number"
                    value={
                      routeData?.instanceDetails?.data?.gameResolution?.value[0]
                    }
                    onChange={(e) => {
                      updateInstanceMutation.mutate({
                        gameResolution: {
                          Set: {
                            type: "Custom",
                            value: [
                              parseInt(e.currentTarget.value, 10),
                              routeData?.instanceDetails?.data?.gameResolution
                                ?.value[1]!
                            ]
                          }
                        },
                        instance: parseInt(params.id, 10)
                      });
                    }}
                  />
                </div>
                <div class="flex items-center gap-4">
                  <div>
                    <Trans key="instance_settings.height" />
                  </div>
                  <Input
                    class="w-24"
                    type="number"
                    value={
                      routeData?.instanceDetails?.data?.gameResolution?.value[1]
                    }
                    onChange={(e) => {
                      updateInstanceMutation.mutate({
                        gameResolution: {
                          Set: {
                            type: "Custom",
                            value: [
                              routeData?.instanceDetails?.data?.gameResolution
                                ?.value[0]!,
                              parseInt(e.currentTarget.value, 10)
                            ]
                          }
                        },
                        instance: parseInt(params.id, 10)
                      });
                    }}
                  />
                </div>
              </div>
            </Show>
          </div>
        </Show>
        <Row>
          <Title description={<Trans key="settings:pre_launch_hook_text" />}>
            <Trans key="settings:pre_launch_hook_title" />
          </Title>
          <RightHandSide>
            <Switch
              checked={
                typeof routeData?.instanceDetails?.data?.preLaunchHook ===
                "string"
              }
              onChange={(e) => {
                updateInstanceMutation.mutate({
                  preLaunchHook: {
                    Set: e.target.checked ? "" : null
                  },
                  instance: parseInt(params.id, 10)
                });
              }}
            />
          </RightHandSide>
        </Row>
        <Show
          when={
            typeof routeData?.instanceDetails?.data?.preLaunchHook === "string"
          }
        >
          <Input
            value={routeData?.instanceDetails?.data?.preLaunchHook || ""}
            onChange={(e) => {
              updateInstanceMutation.mutate({
                preLaunchHook: {
                  Set: e.currentTarget.value.trim()
                },
                instance: parseInt(params.id, 10)
              });
            }}
          />
        </Show>
        <Row>
          <Title description={<Trans key="settings:post_exit_hook_text" />}>
            <Trans key="settings:post_exit_hook_title" />
          </Title>
          <RightHandSide>
            <Switch
              checked={
                typeof routeData?.instanceDetails?.data?.postExitHook ===
                "string"
              }
              onChange={(e) => {
                updateInstanceMutation.mutate({
                  postExitHook: {
                    Set: e.target.checked ? "" : null
                  },
                  instance: parseInt(params.id, 10)
                });
              }}
            />
          </RightHandSide>
        </Row>
        <Show
          when={
            typeof routeData?.instanceDetails?.data?.postExitHook === "string"
          }
        >
          <Input
            value={routeData?.instanceDetails?.data?.postExitHook || ""}
            onChange={(e) => {
              updateInstanceMutation.mutate({
                postExitHook: {
                  Set: e.currentTarget.value.trim()
                },
                instance: parseInt(params.id, 10)
              });
            }}
          />
        </Show>
        <Row>
          <Title description={<Trans key="settings:wrapper_command_text" />}>
            <Trans key="settings:wrapper_command_title" />
          </Title>
          <RightHandSide>
            <Switch
              checked={
                typeof routeData?.instanceDetails?.data?.wrapperCommand ===
                "string"
              }
              onChange={(e) => {
                updateInstanceMutation.mutate({
                  wrapperCommand: {
                    Set: e.target.checked ? "" : null
                  },
                  instance: parseInt(params.id, 10)
                });
              }}
            />
          </RightHandSide>
        </Row>
        <Show
          when={
            typeof routeData?.instanceDetails?.data?.wrapperCommand === "string"
          }
        >
          <Input
            value={routeData?.instanceDetails?.data?.wrapperCommand || ""}
            onChange={(e) => {
              updateInstanceMutation.mutate({
                wrapperCommand: {
                  Set: e.currentTarget.value.trim()
                },
                instance: parseInt(params.id, 10)
              });
            }}
          />
        </Show>
      </RowsContainer>
    </Suspense>
  );
};

export default Settings;
