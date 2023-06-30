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
} from "@gd/ui";
import { useRouteData } from "@solidjs/router";
import { For, Match, Show, Switch } from "solid-js";
import SettingsJavaData from "./settings.java.data";
import { useModal } from "@/managers/ModalsManager";
import { queryClient, rspc } from "@/utils/rspcClient";
import { FEJavaComponentType } from "@gd/core_module/bindings";
import PageTitle from "./components/PageTitle";
import Row from "./components/Row";
import Title from "./components/Title";
import RowsContainer from "./components/RowsContainer";
import RightHandSide from "./components/RightHandSide";

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
        ...newSettings,
      });
    },
  });

  let deleteJavaMutation = rspc.createMutation(["java.deleteJavaVersion"]);

  const mbTotalRAM = () =>
    Math.round(Number(routeData.totalRam.data) / 1024 / 1024);

  const initailJavaArgs = settings.data?.javaCustomArgs;

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
      class="text-darkSlate-50 i-ri:delete-bin-7-fill hover:text-red-500 ease-in-out duration-100 text-xl cursor-pointer transition-color"
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
        <Trans
          key="java.java"
          options={{
            defaultValue: "Java",
          }}
        />
      </PageTitle>
      <RowsContainer>
        <Row>
          <Title>
            <Trans
              key="java.java_memory_title"
              options={{
                defaultValue: "Java Memory",
              }}
            />
          </Title>
          <RightHandSide class="flex w-86 gap box-content gap-12">
            <Slider
              min={256}
              max={mbTotalRAM()}
              steps={1}
              marks={{
                256: "256MB",
                [Math.round(mbTotalRAM() / 2)]: `${Math.round(
                  mbTotalRAM() / 2
                )}MB`,
                [mbTotalRAM()]: `${mbTotalRAM()}MB`,
              }}
              value={settings.data?.xmx}
              onChange={(val) =>
                settingsMutation.mutate({
                  xmx: val,
                })
              }
            />
            <Input
              class="w-26"
              value={settings.data?.xmx}
              onChange={(e) => {
                settingsMutation.mutate({
                  xmx: parseInt(e.currentTarget.value, 10),
                });
              }}
            />
          </RightHandSide>
        </Row>
        <Row class="flex-col items-stretch">
          <Title>
            <Trans
              key="java.java_arguments_title"
              options={{
                defaultValue: "Java Arguments",
              }}
            />
          </Title>
          <div class="flex gap-4 justify-center items-center">
            <Input
              class="w-full"
              value={settings.data?.javaCustomArgs}
              onChange={(e) => {
                settingsMutation.mutate({
                  javaCustomArgs: e.target.value,
                });
              }}
            />
            <Button
              rounded={false}
              type="secondary"
              class="h-10"
              textColor="text-red-500"
              onClick={() => {
                settingsMutation.mutate({
                  javaCustomArgs: initailJavaArgs,
                });
              }}
            >
              <Trans
                key="java.reset_java_args"
                options={{
                  defaultValue: "Reset",
                }}
              />
            </Button>
          </div>
        </Row>
        <Row>
          <Title>
            <Trans
              key="java.auto_handle_java"
              options={{
                defaultValue: "Auto handle java",
              }}
            />
          </Title>
          <RightHandSide>
            <GDSwitch
              checked={settings.data?.autoManageJava}
              onChange={(e) => {
                settingsMutation.mutate({
                  autoManageJava: e.target.checked,
                });
              }}
            />
          </RightHandSide>
        </Row>
        <div class="flex flex-col">
          <Show when={!settings.data?.autoManageJava}>
            <div class="rounded-2xl overflow-hidden">
              <Tabs variant="block">
                <TabList>
                  <Tab>
                    <Trans
                      key="java.manage"
                      options={{
                        defaultValue: "Manage",
                      }}
                    />
                  </Tab>
                  <Tab>
                    <Trans
                      key="java.profiles"
                      options={{
                        defaultValue: "Profiles",
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
                              "We found the following java versions on your pc",
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
                                    defaultValue: "No java available",
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
