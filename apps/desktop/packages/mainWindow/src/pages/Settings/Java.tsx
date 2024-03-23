import { Trans } from "@gd/i18n";
import {
  Button,
  Input,
  Slider,
  Switch as GDSwitch,
  Tab,
  TabList,
  TabPanel,
  Tabs,
  Tooltip,
  Dropdown,
  Popover,
  ContextMenu
} from "@gd/ui";
import { useRouteData } from "@solidjs/router";
import { For, Match, Show, Switch, createMemo } from "solid-js";
import SettingsJavaData from "./settings.java.data";
import { useModal } from "@/managers/ModalsManager";
import { rspc } from "@/utils/rspcClient";
import PageTitle from "./components/PageTitle";
import Row from "./components/Row";
import Title from "./components/Title";
import RowsContainer from "./components/RowsContainer";
import RightHandSide from "./components/RightHandSide";
import { generateSequence } from "@/utils/helpers";
import Center from "./components/Center";
import TruncatedPath from "@/components/TruncatePath";

const Java = () => {
  const routeData: ReturnType<typeof SettingsJavaData> = useRouteData();
  const javasData = () => routeData?.availableJavas;
  const javas = () => javasData()?.data || [];
  const modalsContext = useModal();

  const settings = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }));

  const profileAssignments = rspc.createQuery(() => ({
    queryKey: ["java.systemJavaProfileAssignments"]
  }));

  const settingsMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setSettings"]
  }));

  const updateProfile = rspc.createMutation(() => ({
    mutationKey: ["java.updateJavaProfile"]
  }));

  const deleteProfile = rspc.createMutation(() => ({
    mutationKey: ["java.deleteJavaProfile"]
  }));

  let deleteJavaMutation = rspc.createMutation(() => ({
    mutationKey: ["java.deleteJavaVersion"]
  }));

  const mbTotalRAM = () =>
    Math.round(Number(routeData.totalRam.data) / 1024 / 1024);

  const initialJavaArgs = createMemo((prev: string | undefined) => {
    if (prev) return prev;

    return settings.data?.javaCustomArgs;
  });

  const flattenedAvailableJavas = () =>
    Object.values(routeData.availableJavas.data || {}).reduce(
      (acc, curr) => acc.concat(curr),
      []
    );

  const availableJavasDropdown = () => {
    const results = flattenedAvailableJavas()?.map((java) => {
      return {
        label: (
          <div class="w-full flex flex-col gap-2">
            <div class="flex justify-between">
              <div class="text-white">{java.version}</div>
              <div>{java.type}</div>
            </div>
            <div class="w-full text-left">
              <Tooltip content={java.path}>
                <TruncatedPath originalPath={java.path} />
              </Tooltip>
            </div>
          </div>
        ),
        key: java.id
      };
    });

    return [
      {
        label: "Unassigned",
        key: "unassigned"
      },
      ...results
    ];
  };

  const javaProfiles = () => [
    (routeData.javaProfiles.data || []).filter((profile) => profile.isSystem),
    (routeData.javaProfiles.data || []).filter((profile) => !profile.isSystem)
  ];

  const menuItems = () => [
    {
      icon: "i-ri:pencil-fill",
      label: "Add Managed",
      action: () => {
        modalsContext?.openModal({ name: "addManagedJava" });
      }
    },
    {
      icon: "i-ri:folder-open-fill",
      label: "Add Custom",
      action: () => {
        modalsContext?.openModal({ name: "addCustomJava" });
      }
    }
  ];

  return (
    <>
      <PageTitle>
        <Trans key="settings:Java" />
      </PageTitle>
      <RowsContainer>
        <Row forceContentBelow>
          <Title>
            <Trans key="java.java_memory_title" />
          </Title>
          <Center>
            <Slider
              min={1024}
              max={mbTotalRAM()}
              steps={1024}
              marks={generateSequence(1024, mbTotalRAM())}
              value={settings.data?.xmx}
              onChange={(val) => {
                settingsMutation.mutate({
                  xmx: {
                    Set: val
                  }
                });
              }}
            />
            <Input
              class="w-26"
              value={settings.data?.xmx}
              onChange={(e) => {
                settingsMutation.mutate({
                  xmx: {
                    Set: parseInt(e.currentTarget.value, 10)
                  }
                });
              }}
            />
          </Center>
        </Row>
        <Row class="flex-col items-stretch">
          <Title>
            <Trans key="java.java_arguments_title" />
          </Title>
          <div class="flex gap-4 justify-center items-center">
            <Input
              class="w-full"
              value={settings.data?.javaCustomArgs}
              onChange={(e) => {
                settingsMutation.mutate({
                  javaCustomArgs: {
                    Set: e.target.value
                  }
                });
              }}
            />
            <Tooltip content={<Trans key="tooltip.undo" />}>
              <Button
                type="secondary"
                class="h-10"
                size="small"
                onClick={() => {
                  settingsMutation.mutate({
                    javaCustomArgs: {
                      Set: initialJavaArgs() || ""
                    }
                  });
                }}
              >
                <i class="w-5 h-5 i-ri:arrow-go-back-fill" />
              </Button>
            </Tooltip>
            <Tooltip content={<Trans key="tooltip.reset" />}>
              <Button
                type="secondary"
                class="h-10"
                size="small"
                onClick={() => {
                  settingsMutation.mutate({
                    javaCustomArgs: {
                      Set: ""
                    }
                  });
                }}
              >
                <i class="w-5 h-5 i-ri:close-fill" />
              </Button>
            </Tooltip>
          </div>
        </Row>
        <Row>
          <Title
            description={
              <Trans key="java.auto_manage_java_system_profiles_text" />
            }
          >
            <Trans key="java.auto_manage_java_system_profiles" />
          </Title>
          <RightHandSide>
            <GDSwitch
              checked={settings.data?.autoManageJavaSystemProfiles}
              onChange={(e) => {
                settingsMutation.mutate({
                  autoManageJavaSystemProfiles: {
                    Set: e.target.checked
                  }
                });
              }}
            />
          </RightHandSide>
        </Row>
        <div class="flex flex-col">
          <div class="overflow-hidden">
            <Tabs>
              <TabList heightClass="h-14">
                <Tab class="w-1/2" centerContent>
                  <Trans key="java.manage_paths" />
                </Tab>
                <Tab class="w-1/2" centerContent>
                  <Trans key="java.manage_profiles" />
                </Tab>
              </TabList>
              <TabPanel>
                <div class="h-full p-4 min-h-96">
                  <div class="flex justify-between items-center mb-4">
                    <div>
                      <div class="m-0 text-sm font-normal">
                        <Trans key="java.java_description_text" />
                      </div>
                      <div class="m-0 text-sm font-normal">
                        <Trans key="java.java_description_local_text" />
                      </div>
                      <div class="m-0 text-sm font-normal">
                        <Trans key="java.java_description_managed_text" />
                      </div>
                      <div class="m-0 text-sm font-normal">
                        <Trans key="java.java_description_custom_text" />
                      </div>
                    </div>
                    <ContextMenu menuItems={menuItems()} trigger="click">
                      <Button type="secondary" size="small">
                        <div class="text-xl i-ri:add-fill" />
                      </Button>
                    </ContextMenu>
                  </div>
                  <div class="flex flex-col gap-4">
                    <For each={Object.entries(javas())}>
                      {([javaVersion, obj]) => (
                        <div class="rounded-xl border-1 border-solid border-darkSlate-600">
                          <h3 class="px-4">
                            <Trans
                              key="java.java_version_number"
                              options={{
                                version: javaVersion
                              }}
                            />
                          </h3>
                          <Show when={obj.length > 0}>
                            <div class="flex flex-col gap-2">
                              <For each={obj}>
                                {(java) => {
                                  const usedInNProfiles = () =>
                                    (routeData.javaProfiles.data || []).filter(
                                      (item) => item.javaId === java.id
                                    );

                                  return (
                                    <div class="flex justify-between rounded-md py-2 px-4 hover:bg-darkSlate-600">
                                      <div class="flex flex-col gap-2 w-full flex-1 min-w-0">
                                        <div class="flex justify-between">
                                          <div class="flex items-center gap-2">
                                            <div class="text-white flex items-center gap-2">
                                              <div>{java.version}</div>
                                              <Switch>
                                                <Match when={java.isValid}>
                                                  <Tooltip content="This java path works and is valid">
                                                    <div class="flex i-ri:checkbox-circle-fill text-emerald-500" />
                                                  </Tooltip>
                                                </Match>
                                                <Match when={!java.isValid}>
                                                  <Tooltip content="This java path doesn't seem to work">
                                                    <div class="flex i-ri:error-warning-fill text-yellow-500" />
                                                  </Tooltip>
                                                </Match>
                                              </Switch>
                                            </div>
                                            <Show
                                              when={
                                                usedInNProfiles().length > 0
                                              }
                                            >
                                              <div class="h-2/3 w-px bg-darkSlate-400 mr-2" />
                                              <Popover
                                                content={
                                                  <div class="p-4">
                                                    <h3>
                                                      <Trans key="settings:used_in_the_following_profiles" />
                                                    </h3>
                                                    <ul class="flex flex-col gap-2">
                                                      <For
                                                        each={usedInNProfiles()}
                                                      >
                                                        {(profile) => (
                                                          <li class="text-lightSlate-600">
                                                            {profile.name}
                                                          </li>
                                                        )}
                                                      </For>
                                                    </ul>
                                                  </div>
                                                }
                                              >
                                                <div class="text-sm underline">
                                                  <Trans
                                                    key="settings:used_in_counted_profiles"
                                                    options={{
                                                      count:
                                                        usedInNProfiles().length
                                                    }}
                                                  />
                                                </div>
                                              </Popover>
                                            </Show>
                                          </div>
                                        </div>
                                        <div class="flex justify-between">
                                          <div class="flex-1 text-xs text-lightSlate-700 overflow-hidden whitespace-nowrap">
                                            <Tooltip content={java.path}>
                                              <TruncatedPath
                                                originalPath={java.path}
                                              />
                                            </Tooltip>
                                          </div>
                                        </div>
                                      </div>
                                      <div class="flex items-center ml-2">
                                        <Show
                                          when={
                                            java.type === "custom" ||
                                            java.type === "managed"
                                          }
                                        >
                                          <div
                                            class="text-lightSlate-800 hover:text-red-400 ease-in-out duration-100 text-lg transition-color i-ri:delete-bin-7-fill"
                                            onClick={() =>
                                              deleteJavaMutation.mutate(java.id)
                                            }
                                          />
                                        </Show>
                                      </div>
                                    </div>
                                  );
                                }}
                              </For>
                            </div>
                          </Show>
                          <Show when={obj.length === 0}>
                            <p>
                              <Trans key="java.no_found_java_text" />
                            </p>
                          </Show>
                        </div>
                      )}
                    </For>
                  </div>
                </div>
              </TabPanel>
              <TabPanel>
                <div class="h-full p-4 flex flex-col gap-4 min-h-96">
                  <div class="flex justify-between items-center mb-4">
                    <h2 class="m-0 text-sm font-normal">
                      <Trans key="java.profiles_description_text" />
                    </h2>
                  </div>
                  <For each={javaProfiles()}>
                    {(profiles, i) => (
                      <div class="rounded-xl border-1 border-solid border-darkSlate-600">
                        <div class="flex items-center justify-between px-4">
                          <h3>
                            <Switch>
                              <Match when={i() === 0}>
                                <Trans key="settings:system_profiles" />
                              </Match>
                              <Match when={i() === 1}>
                                <Trans key="settings:custom_profiles" />
                              </Match>
                            </Switch>
                          </h3>
                          <Show when={i() === 1}>
                            <Button
                              type="secondary"
                              size="small"
                              onClick={() => {
                                modalsContext?.openModal({
                                  name: "javaProfileCreation"
                                });
                              }}
                            >
                              <div class="text-xl i-ri:add-fill" />
                            </Button>
                          </Show>
                        </div>
                        <For each={profiles}>
                          {(profile) => {
                            const id = flattenedAvailableJavas()?.find(
                              (java) => java.id === profile.javaId
                            )?.id;

                            return (
                              <div class="px-4 py-2 flex justify-between items-center hover:bg-darkSlate-600">
                                <h3 class="m-0 text-lightSlate-700 text-sm">
                                  {profile.name}
                                </h3>
                                <div class="m-0 flex items-center gap-4">
                                  <Dropdown
                                    class="w-70"
                                    value={id || "unassigned"}
                                    options={availableJavasDropdown()}
                                    disabled={
                                      profile.isSystem &&
                                      settings.data
                                        ?.autoManageJavaSystemProfiles
                                    }
                                    onChange={(option) => {
                                      updateProfile.mutate({
                                        profileName: profile.name,
                                        javaId:
                                          option.key.toString() === "unassigned"
                                            ? null
                                            : option.key.toString()
                                      });
                                    }}
                                  />
                                  <Show when={i() === 1}>
                                    <div
                                      class="text-lightSlate-800 hover:text-red-400 ease-in-out duration-100 text-lg transition-color i-ri:delete-bin-7-fill"
                                      onClick={() => {
                                        deleteProfile.mutate(profile.name);
                                      }}
                                    />
                                  </Show>
                                  <Show when={profile.isSystem}>
                                    <Popover
                                      content={
                                        <div class="p-4 flex flex-col gap-8 max-w-100 h-auto">
                                          <div>
                                            <Trans key="settings:profile_used_in_mc_versions" />
                                          </div>
                                          <div class="p-4 flex flex-wrap items-start justify-start content-start gap-4 overflow-y-auto h-70">
                                            <For
                                              each={
                                                profileAssignments.data?.[
                                                  profile.name
                                                ] || []
                                              }
                                            >
                                              {(assignment) => (
                                                <div class="text-lightSlate-800">
                                                  {assignment}
                                                </div>
                                              )}
                                            </For>
                                          </div>
                                        </div>
                                      }
                                    >
                                      <div class="i-ri:information-fill text-lg text-lightSlate-800 hover:text-lightSlate-100 ease-in-out duration-100 transition-color" />
                                    </Popover>
                                  </Show>
                                </div>
                              </div>
                            );
                          }}
                        </For>
                      </div>
                    )}
                  </For>
                </div>
              </TabPanel>
            </Tabs>
          </div>
        </div>
      </RowsContainer>
    </>
  );
};

export default Java;
