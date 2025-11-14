import { generateSequence } from "@/utils/helpers"
import { port, queryClient, rspc } from "@/utils/rspcClient"
import { Trans, useTransContext } from "@gd/i18n"
import {
  Button,
  Select,
  SelectTrigger,
  SelectContent,
  SelectItem,
  SelectValue,
  Input,
  Radio,
  Slider,
  Switch
} from "@gd/ui"
import { useParams, useRouteData } from "@solidjs/router"
import fetchData from "../../instance.data"
import { Match, Show, createMemo, Switch as SolidSwitch } from "solid-js"
import { InstanceDetails } from "@gd/core_module/bindings"
import Title from "@/pages/Settings/components/Title"
import Row from "@/pages/Settings/components/Row"
import RowsContainer from "@/pages/Settings/components/RowsContainer"
import RightHandSide from "@/pages/Settings/components/RightHandSide"
import { useModal } from "@/managers/ModalsManager"
import JavaPathAutoComplete from "@/components/JavaPathAutoComplete"
import useSearchContext from "@/components/SearchInputContext"

const Settings = () => {
  const [t] = useTransContext()
  const modalsContext = useModal()
  const searchContext = useSearchContext()
  const params = useParams()
  const updateInstanceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.updateInstance"],
    onMutate: async (variables) => {
      // Cancel outgoing refetches to prevent overwriting optimistic update
      await queryClient.cancelQueries({
        queryKey: ["instance.getInstanceDetails", parseInt(params.id, 10)]
      })

      // Snapshot current state for rollback on error
      const previousDetails = queryClient.getQueryData<InstanceDetails>([
        "instance.getInstanceDetails",
        parseInt(params.id, 10)
      ])

      // Optimistically update cache
      queryClient.setQueryData(
        ["instance.getInstanceDetails", parseInt(params.id, 10)],
        (old: InstanceDetails | undefined) => {
          if (!old) return old
          return {
            ...old,
            name:
              variables.name?.Set !== undefined ? variables.name.Set : old.name,
            modpack:
              variables.modpackLocked?.Set !== undefined
                ? variables.modpackLocked.Set === null
                  ? undefined // Unpair - remove modpack
                  : old.modpack
                    ? { ...old.modpack, locked: variables.modpackLocked.Set }
                    : old.modpack
                : old.modpack,
            memory:
              variables.memory?.Set !== undefined
                ? variables.memory.Set
                : old.memory,
            javaOverride:
              variables.javaOverride?.Set !== undefined
                ? variables.javaOverride.Set
                : old.javaOverride,
            extraJavaArgs:
              variables.extraJavaArgs?.Set !== undefined
                ? variables.extraJavaArgs.Set
                : old.extraJavaArgs,
            globalJavaArgs:
              variables.globalJavaArgs?.Set !== undefined
                ? variables.globalJavaArgs.Set
                : old.globalJavaArgs,
            gameResolution:
              variables.gameResolution?.Set !== undefined
                ? variables.gameResolution.Set
                : old.gameResolution,
            preLaunchHook:
              variables.preLaunchHook?.Set !== undefined
                ? variables.preLaunchHook.Set
                : old.preLaunchHook,
            postExitHook:
              variables.postExitHook?.Set !== undefined
                ? variables.postExitHook.Set
                : old.postExitHook,
            wrapperCommand:
              variables.wrapperCommand?.Set !== undefined
                ? variables.wrapperCommand.Set
                : old.wrapperCommand
          }
        }
      )

      return { previousDetails }
    },
    onError: (_err, _variables, context) => {
      // Rollback to previous state on error
      if (context?.previousDetails) {
        queryClient.setQueryData(
          ["instance.getInstanceDetails", parseInt(params.id, 10)],
          context.previousDetails
        )
      }
    },
    onSettled: () => {
      // Refetch to ensure cache is in sync with backend
      queryClient.invalidateQueries({
        queryKey: ["instance.getInstanceDetails", parseInt(params.id, 10)]
      })
    }
  }))

  const getAllProfiles = rspc.createQuery(() => ({
    queryKey: ["java.getJavaProfiles"]
  }))

  const routeData: ReturnType<typeof fetchData> = useRouteData()

  const initialJavaArgs = createMemo((prev: string | null) => {
    if (prev) return prev

    return routeData?.instanceDetails?.data?.extraJavaArgs as string | null
  }, null)

  const mbTotalRAM = () => Number(routeData.totalRam.data) / 1024 / 1024

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
      case "Standard:854x480":
        return "854 x 480 (100%)"
      case "Standard:1046x588":
        return "1046 x 588 (150%)"
      case "Standard:1208x679":
        return "1208 x 679 (200%)"
      case "Standard:1479x831":
        return "1479 x 831 (300%)"
      case "custom":
        return t("ui:_trn_custom")
      default:
        return key
    }
  }

  const gameResolutionDropdownKey = () => {
    if (routeData?.instanceDetails?.data?.gameResolution?.type === "Standard") {
      const gameResolution =
        routeData?.instanceDetails?.data?.gameResolution.value.join("x")
      return `Standard:${gameResolution}`
    }

    return "custom"
  }

  const javaOverrideType = () => {
    if ("Profile" in (routeData?.instanceDetails?.data?.javaOverride || {})) {
      return "Profile"
    }

    return "Path"
  }

  const javaSelectedProfile = () => {
    const profileName = (routeData?.instanceDetails?.data?.javaOverride as any)
      ?.Profile

    if (!profileName) return null

    return getAllProfiles.data?.find((profile) => profile.name === profileName)
      ?.name
  }

  return (
    <RowsContainer>
      <Show when={routeData?.instanceDetails?.data?.modpack}>
        <Row>
          <Title>
            <Trans key="instances:_trn_instance_settings.modpack_info" />
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
                  searchContext?.setSelectedInstanceId(parseInt(params.id, 10))
                  // modalsContext?.openModal(
                  //   {
                  //     name: "unlock_confirmation"
                  //   },
                  //   {
                  //     instanceState: "unlock",
                  //     instanceId: parseInt(params.id, 10)
                  //   }
                  // )
                  updateInstanceMutation.mutate({
                    modpackLocked: {
                      Set: false
                    },
                    instance: parseInt(params.id, 10)
                  })
                }}
              >
                <i class="i-hugeicons:lock h-5 w-5" />
                <Trans key="instances:_trn_instance_settings.unlock" />
              </Button>
            </Show>
            <Show when={!routeData.instanceDetails.data?.modpack?.locked}>
              <div class="flex items-center gap-2">
                <i class="i-hugeicons:lock-key-open h-5 w-5" />
                <Trans key="instances:_trn_instance_settings.unlocked" />
              </div>
            </Show>
            <Button
              type="outline"
              onClick={() => {
                searchContext?.setSelectedInstanceId(parseInt(params.id, 10))
                modalsContext?.openModal(
                  {
                    name: "unpair_confirmation"
                  },
                  {
                    instanceState: "unpair",
                    instanceId: parseInt(params.id, 10)
                  }
                )
              }}
            >
              <i class="i-hugeicons:git-branch h-5 w-5" />
              <Trans key="instances:_trn_instance_settings.unpair" />
            </Button>
            <Button
              type="outline"
              onClick={() => {
                modalsContext?.openModal(
                  {
                    name: "modpack_version_update"
                  },
                  {
                    instanceId: parseInt(params.id, 10)
                  }
                )
              }}
            >
              <i class="i-hugeicons:arrow-left-right h-5 w-5" />
              <Trans key="instances:_trn_instance_settings.change_modpack_version" />
            </Button>
          </div>
        </div>
      </Show>
      <Row>
        <Title>
          <Trans key="java:_trn_instance_settings.java_path_profile" />
        </Title>
        <RightHandSide>
          <Switch
            checked={!!routeData?.instanceDetails?.data?.javaOverride}
            onChange={(v) => {
              updateInstanceMutation.mutate({
                javaOverride: {
                  Set: v.target.checked
                    ? ({
                        Profile:
                          routeData.instanceDetails.data?.requiredJavaProfile
                      } as any)
                    : null
                },
                instance: parseInt(params.id, 10)
              })
            }}
          />
        </RightHandSide>
      </Row>
      <Show when={routeData?.instanceDetails?.data?.javaOverride !== null}>
        <Radio.group
          value={javaOverrideType()}
          buttonStyle="button"
          onChange={(v) => {
            const payload =
              v?.toString() === "Path"
                ? {
                    Path: null
                  }
                : {
                    Profile: routeData.instanceDetails.data?.requiredJavaProfile
                  }

            updateInstanceMutation.mutate({
              javaOverride: {
                Set: payload as any
              },
              instance: parseInt(params.id, 10)
            })
          }}
          options={[
            {
              value: "Path",
              label: t("ui:_trn_path")
            },
            {
              value: "Profile",
              label: t("ui:_trn_profile")
            }
          ]}
        />
        <SolidSwitch>
          <Match when={javaOverrideType() === "Path"}>
            <div class="min-w-100 max-w-2/3">
              <JavaPathAutoComplete
                defaultValue={
                  (routeData?.instanceDetails?.data?.javaOverride as any)?.Path
                }
                updateValueOnlyOnBlur
                updateValue={(id, value) => {
                  updateInstanceMutation.mutate({
                    javaOverride: {
                      Set: {
                        Path: value || null
                      }
                    },
                    instance: parseInt(params.id, 10)
                  })
                }}
              />
            </div>
          </Match>
          <Match when={javaOverrideType() === "Profile"}>
            <div class="flex gap-2">
              <div class="text-lightSlate-700">
                <Trans key="java:_trn_this_instance_requires" />
              </div>
              <div class="text-lightSlate-100">
                {routeData.instanceDetails.data?.requiredJavaProfile}
              </div>
            </div>
            <div class="flex items-center">
              <Select
                value={javaSelectedProfile()}
                placeholder={t("placeholders:_trn_select_java_profile")}
                options={
                  getAllProfiles.data?.map((profile) => profile.name) || []
                }
                onChange={(option) => {
                  if (option) {
                    updateInstanceMutation.mutate({
                      javaOverride: {
                        Set: {
                          Profile: option
                        }
                      },
                      instance: parseInt(params.id, 10)
                    })
                  }
                }}
                itemComponent={(props) => (
                  <SelectItem item={props.item}>
                    {props.item.rawValue}
                  </SelectItem>
                )}
              >
                <SelectTrigger class="min-w-100 max-w-2/3">
                  <SelectValue<string>>
                    {(state) =>
                      state.selectedOption() ||
                      t("placeholders:_trn_select_java_profile")
                    }
                  </SelectValue>
                </SelectTrigger>
                <SelectContent />
              </Select>
              <Button
                type="primary"
                onClick={() => {
                  modalsContext?.openModal({
                    name: "javaProfileCreation"
                  })
                }}
              >
                <Trans key="java:_trn_add_new_profile" />
              </Button>
            </div>
          </Match>
        </SolidSwitch>
      </Show>
      <Row>
        <Title>
          <Trans key="java:_trn_instance_settings.java_memory_title" />
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
              })
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
                return
              }
              queryClient.setQueryData(
                ["instance.getInstanceDetails"],
                (oldData: InstanceDetails | undefined) => {
                  if (!oldData) return
                  oldData.memory = {
                    max_mb: val,
                    min_mb: val
                  }
                  return oldData
                }
              )
            }}
            OnRelease={(val) => {
              if (
                !val ||
                routeData?.instanceDetails.data?.memory?.max_mb === val
              ) {
                return
              }

              updateInstanceMutation.mutate({
                memory: { Set: { max_mb: val, min_mb: val } },
                instance: parseInt(params.id, 10)
              })
            }}
          />
        </div>
      </Show>

      <Row>
        <Title>
          <Trans key="java:_trn_instance_settings.java_arguments_title" />
        </Title>
        <Switch
          checked={
            routeData?.instanceDetails?.data?.extraJavaArgs !== null &&
            routeData?.instanceDetails?.data?.extraJavaArgs !== undefined
          }
          onChange={(e) => {
            const checked = e.target.checked

            updateInstanceMutation.mutate({
              extraJavaArgs: { Set: checked ? "" : null },
              instance: parseInt(params.id, 10)
            })
          }}
        />
      </Row>
      <Show
        when={
          routeData?.instanceDetails?.data?.extraJavaArgs !== null &&
          routeData?.instanceDetails?.data?.extraJavaArgs !== undefined
        }
      >
        <div class="-mt-8 flex w-full items-center justify-between">
          <h5 class="text-lightSlate-700">
            <Trans key="java:_trn_instance_settings.prepend_global_java_args" />
          </h5>
          <Switch
            checked={routeData?.instanceDetails?.data?.globalJavaArgs}
            onChange={(e) => {
              const checked = e.target.checked

              updateInstanceMutation.mutate({
                globalJavaArgs: { Set: checked },
                instance: parseInt(params.id, 10)
              })
            }}
          />
        </div>
        <div class="flex w-full items-center gap-4">
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
              })
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
              })
            }}
          >
            <i class="i-hugeicons:arrow-turn-backward h-5 w-5" />
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
              })
            }}
          >
            <i class="i-hugeicons:cancel-01 h-5 w-5" />
          </Button>
        </div>
      </Show>
      <Row>
        <Title
          description={
            <Trans key="instances:_trn_instance_settings.game_resolution_text" />
          }
        >
          <Trans key="instances:_trn_instance_settings.game_resolution_title" />
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
              })
            }}
          />
        </RightHandSide>
      </Row>
      <Show when={routeData?.instanceDetails?.data?.gameResolution}>
        <div class="flex gap-4">
          <Select
            value={gameResolutionDropdownKey()}
            placeholder={t("settings:_trn_resolution_presets")}
            options={[...templateGameResolution(), "custom"]}
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
              } else {
                const [width, height] = key.toString().split(":")[1].split("x")
                value = {
                  type: "Standard",
                  value: [parseInt(width, 10), parseInt(height, 10)]
                }
              }

              updateInstanceMutation.mutate({
                gameResolution: {
                  Set: value
                },
                instance: parseInt(params.id, 10)
              })
            }}
            itemComponent={(props) => (
              <SelectItem item={props.item}>
                {getResolutionLabel(props.item.rawValue)}
              </SelectItem>
            )}
          >
            <SelectTrigger>
              <SelectValue<string>>
                {(state) => getResolutionLabel(state.selectedOption() || "")}
              </SelectValue>
            </SelectTrigger>
            <SelectContent />
          </Select>
          <Show
            when={
              routeData?.instanceDetails?.data?.gameResolution?.type ===
              "Custom"
            }
          >
            <div class="flex gap-4">
              <div class="flex items-center gap-4">
                <div>
                  <Trans key="instances:_trn_instance_settings.width" />
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
                    })
                  }}
                />
              </div>
              <div class="flex items-center gap-4">
                <div>
                  <Trans key="instances:_trn_instance_settings.height" />
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
                    })
                  }}
                />
              </div>
            </div>
          </Show>
        </div>
      </Show>
      <Row>
        <Title description={<Trans key="settings:_trn_pre_launch_hook_text" />}>
          <Trans key="settings:_trn_pre_launch_hook_title" />
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
              })
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
            })
          }}
        />
      </Show>
      <Row>
        <Title description={<Trans key="settings:_trn_post_exit_hook_text" />}>
          <Trans key="settings:_trn_post_exit_hook_title" />
        </Title>
        <RightHandSide>
          <Switch
            checked={
              typeof routeData?.instanceDetails?.data?.postExitHook === "string"
            }
            onChange={(e) => {
              updateInstanceMutation.mutate({
                postExitHook: {
                  Set: e.target.checked ? "" : null
                },
                instance: parseInt(params.id, 10)
              })
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
            })
          }}
        />
      </Show>
      <Row>
        <Title description={<Trans key="settings:_trn_wrapper_command_text" />}>
          <Trans key="settings:_trn_wrapper_command_title" />
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
              })
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
            })
          }}
        />
      </Show>
    </RowsContainer>
  )
}

export default Settings
