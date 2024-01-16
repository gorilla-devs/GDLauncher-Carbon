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
  Tooltip
} from "@gd/ui";
import { useRouteData } from "@solidjs/router";
import { For, Match, Show, Switch, createMemo } from "solid-js";
import SettingsJavaData from "./settings.java.data";
import { useModal } from "@/managers/ModalsManager";
import { queryClient, rspc } from "@/utils/rspcClient";
import { FEJavaComponentType } from "@gd/core_module/bindings";
import PageTitle from "./components/PageTitle";
import Row from "./components/Row";
import Title from "./components/Title";
import RowsContainer from "./components/RowsContainer";
import RightHandSide from "./components/RightHandSide";
import { generateSequence } from "@/utils/helpers";
import Center from "./components/Center";

const Java = () => {
  const routeData: ReturnType<typeof SettingsJavaData> = useRouteData();
  const javasData = () => routeData?.availableJavas;
  const javas = () => javasData()?.data || [];
  const modalsContext = useModal();

  const settings = rspc.createQuery(() => ["settings.getSettings"]);

  const settingsMutation = rspc.createMutation(["settings.setSettings"], {
    onMutate: (newSettings) => {
      queryClient.setQueryData(["settings.getSettings"], {
        ...settings?.data,
        ...newSettings
      });
    }
  });

  let deleteJavaMutation = rspc.createMutation(["java.deleteJavaVersion"]);

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

  const javaInProfile = (id: string) => {
    return (routeData.javaProfiles.data || []).some(
      (item) => item.javaId === id
    );
  };

  const DeleteIcon = (props: { id: string }) => (
    <div
      class="text-darkSlate-50 hover:text-red-500 ease-in-out duration-100 text-xl cursor-pointer transition-color i-ri:delete-bin-7-fill"
      onClick={() => deleteJavaMutation.mutate(props.id)}
    />
  );

  const mapJavaTypeToAction = (type: FEJavaComponentType, id: string) => {
    return (
      <>
        <Show when={type === "custom" || type === "managed"}>
          <div class="flex gap-2">
            <Switch>
              <Match when={type === "custom"}>
                <DeleteIcon id={id} />
                <div class="text-darkSlate-50 transition-color ease-in-out duration-100 text-xl cursor-pointer i-ri:pencil-fill hover:darkSlate-200" />
              </Match>
              <Match when={type === "managed"}>
                <DeleteIcon id={id} />
              </Match>
            </Switch>
          </div>
        </Show>
      </>
    );
  };

  return (
    <>
      <PageTitle>
        <Trans key="settings:Java" />
      </PageTitle>
      <RowsContainer>
        <Row forceContentBelow>
          <Title>
            <Trans
              key="java.java_memory_title"
              options={{
                defaultValue: "Java Memory"
              }}
            />
          </Title>
          <Center>
            <Slider
              min={256}
              max={mbTotalRAM()}
              steps={1000}
              marks={generateSequence(2048, mbTotalRAM())}
              value={settings.data?.xmx}
              onChange={(val) =>
                settingsMutation.mutate({
                  xmx: val
                })
              }
            />
            <Input
              class="w-26"
              value={settings.data?.xmx}
              onChange={(e) => {
                settingsMutation.mutate({
                  xmx: parseInt(e.currentTarget.value, 10)
                });
              }}
            />
          </Center>
        </Row>
        <Row class="flex-col items-stretch">
          <Title>
            <Trans
              key="java.java_arguments_title"
              options={{
                defaultValue: "Java Arguments"
              }}
            />
          </Title>
          <div class="flex gap-4 justify-center items-center">
            <Input
              class="w-full"
              value={settings.data?.javaCustomArgs}
              onChange={(e) => {
                settingsMutation.mutate({
                  javaCustomArgs: e.target.value
                });
              }}
            />
            <Tooltip content={<Trans key="tooltip.undo" />}>
              <Button
                rounded={false}
                type="secondary"
                class="h-10"
                size="small"
                onClick={() => {
                  settingsMutation.mutate({
                    javaCustomArgs: initialJavaArgs()
                  });
                }}
              >
                <i class="w-5 h-5 i-ri:arrow-go-back-fill" />
              </Button>
            </Tooltip>
            <Tooltip content={<Trans key="tooltip.reset" />}>
              <Button
                rounded={false}
                type="secondary"
                class="h-10"
                size="small"
                onClick={() => {
                  settingsMutation.mutate({
                    javaCustomArgs: ""
                  });
                }}
              >
                <i class="w-5 h-5 i-ri:close-fill" />
              </Button>
            </Tooltip>
          </div>
        </Row>
        <Row>
          <Title>
            <Trans
              key="java.auto_handle_java"
              options={{
                defaultValue: "Auto handle java"
              }}
            />
          </Title>
          <RightHandSide>
            <GDSwitch
              checked={settings.data?.autoManageJava}
              onChange={(e) => {
                settingsMutation.mutate({
                  autoManageJava: e.target.checked
                });
              }}
            />
          </RightHandSide>
        </Row>
        <div class="flex flex-col">
          <Show when={!settings.data?.autoManageJava}>
            <div class="overflow-hidden">
              <Tabs>
                <TabList heightClass="h-14">
                  <Tab class="w-1/2" centerContent>
                    <Trans
                      key="java.manage"
                      options={{
                        defaultValue: "Manage"
                      }}
                    />
                  </Tab>
                  <Tab class="w-1/2" centerContent>
                    <Trans
                      key="java.profiles"
                      options={{
                        defaultValue: "Profiles"
                      }}
                    />
                  </Tab>
                </TabList>
                <TabPanel>
                  <div class="h-full bg-darkSlate-900 p-4 min-h-96">
                    <div class="flex justify-between items-center mb-4">
                      <h2 class="m-0 text-sm font-normal">
                        <Trans
                          key="java.found_java_text"
                          options={{
                            defaultValue:
                              "We found the following java versions on your pc"
                          }}
                        />
                      </h2>
                      <Button
                        rounded={false}
                        type="secondary"
                        size="small"
                        onClick={() => {
                          modalsContext?.openModal({ name: "addJava" });
                        }}
                      >
                        <div class="text-xl text-darkSlate-500 i-ri:add-fill" />
                      </Button>
                    </div>
                    <div class="flex flex-col gap-4">
                      <For each={Object.entries(javas())}>
                        {([javaVersion, obj]) => (
                          <div class="p-4 rounded-xl border-1 border-solid border-darkSlate-600">
                            <h3 class="m-0 mb-4">{javaVersion}</h3>
                            <Show when={obj.length > 0}>
                              <div class="flex flex-col gap-4">
                                <For each={obj}>
                                  {(java) => (
                                    <div class="border-1 border-solid border-darkSlate-600 flex justify-between items-center bg-darkSlate-700 rounded-lg px-4 py-2">
                                      <span class="text-xs text-darkSlate-100">
                                        {java.path}
                                      </span>
                                      <div class="flex gap-2 justify-center items-center">
                                        <span>{java.type}</span>
                                        {mapJavaTypeToAction(
                                          java.type,
                                          java.id
                                        )}
                                        <Show when={javaInProfile(java.id)}>
                                          <div class="text-green-500 i-ri:checkbox-circle-fill" />
                                        </Show>
                                      </div>
                                    </div>
                                  )}
                                </For>
                              </div>
                            </Show>
                            <Show when={obj.length === 0}>
                              <p>
                                <Trans
                                  key="java.no_found_java_text"
                                  options={{
                                    defaultValue: "No java available"
                                  }}
                                />
                              </p>
                            </Show>
                          </div>
                        )}
                      </For>
                    </div>
                  </div>
                </TabPanel>
                <TabPanel>
                  <div class="bg-darkSlate-900 h-full p-4 flex flex-col gap-4 min-h-96">
                    <For each={routeData.javaProfiles.data}>
                      {(profile) => {
                        const path = flattenedAvailableJavas()?.find(
                          (java) => java.id === profile.javaId
                        )?.path;
                        return (
                          <div class="rounded-xl border-1 border-solid border-darkSlate-600 p-4 flex justify-between items-center">
                            <h3 class="m-0">{profile.name}</h3>
                            <span class="m-0">
                              <Switch>
                                <Match when={path}>{path}</Match>
                                <Match when={!path}>-</Match>
                              </Switch>
                            </span>
                          </div>
                        );
                      }}
                    </For>
                  </div>
                </TabPanel>
              </Tabs>
            </div>
          </Show>
        </div>
      </RowsContainer>
    </>
  );
};

export default Java;
