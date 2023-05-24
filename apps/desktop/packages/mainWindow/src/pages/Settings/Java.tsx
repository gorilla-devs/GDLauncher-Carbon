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

const Java = () => {
  const routeData: ReturnType<typeof SettingsJavaData> = useRouteData();
  const javasData = () => routeData?.availableJavas;
  const javas = () => javasData()?.data || [];
  const modalsContext = useModal();

  let setSettingsMutation = rspc.createMutation(["settings.setSettings"], {
    onMutate: (newTheme) => {
      queryClient.setQueryData(["settings.setSettings"], newTheme);
    },
  });

  const mbTotalRAM = () => Number(routeData.totalRam.data) / 1024 / 1024;

  const generateSequence = (
    min: number,
    max: number
  ): Record<number, string> => {
    let current = min;
    const sequence: Record<number, string> = {};

    while (current <= max) {
      sequence[current] = `${current} MB`;
      current *= 2;
    }

    return sequence;
  };

  const initailJavaArgs = routeData.settings.data?.javaCustomArgs;

  const flattenedAvailableJavas = () =>
    Object.values(routeData.availableJavas.data || {}).reduce(
      (acc, curr) => acc.concat(curr),
      []
    );

  return (
    <div class="bg-darkSlate-800 w-full h-auto flex flex-col pt-5 px-6 box-border pb-10">
      <h2 class="m-0 mb-7 text-4">
        <Trans
          key="java.java"
          options={{
            defaultValue: "Java",
          }}
        />
      </h2>

      <Show when={routeData.settings.data}>
        <div class="mb-4">
          <h5 class="m-0 mb-4">
            <Trans
              key="java.java_memory_title"
              options={{
                defaultValue: "Java Memory",
              }}
            />
          </h5>
          <div class="flex justify-center px-3">
            <Slider
              min={0}
              max={mbTotalRAM()}
              steps={1}
              value={routeData.settings.data?.xmx}
              marks={generateSequence(1024, mbTotalRAM())}
              onChange={(val) =>
                setSettingsMutation.mutate({
                  xmx: val,
                })
              }
            />
          </div>
        </div>
      </Show>
      <div class="mb-4">
        <h5 class="m-0 mb-4">
          <Trans
            key="java.java_arguments_title"
            options={{
              defaultValue: "Java Arguments",
            }}
          />
        </h5>
        <div class="flex w-full gap-4 items-center">
          <Input
            class="w-full"
            value={routeData.settings.data?.javaCustomArgs}
            onChange={(e) => {
              setSettingsMutation.mutate({
                javaCustomArgs: e.target.value,
              });
            }}
          />
          <Button
            rounded={false}
            variant="secondary"
            class="h-10"
            textColor="text-red-500"
            onClick={() => {
              setSettingsMutation.mutate({
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
      </div>
      <div class="h-full flex justify-between mb-4 mt-10">
        <h2 class="mt-0 text-sm">
          <Trans
            key="java.auto_handle_java"
            options={{
              defaultValue: "Auto handle java",
            }}
          />
        </h2>
        <GDSwitch
          checked={routeData.settings.data?.autoManageJava}
          onChange={(e) => {
            console.log("EEEE", e.target.checked);
            setSettingsMutation.mutate({ autoManageJava: e.target.checked });
          }}
        />
      </div>
      <div class="flex flex-col">
        <div class="overflow-hidden rounded-2xl">
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
              <div class="bg-darkSlate-900 h-full p-4 min-h-96">
                <div class="flex justify-between items-center mb-4">
                  <h2 class="m-0 text-sm font-normal">
                    <Trans
                      key="java.found_java_text"
                      options={{
                        defaultValue:
                          "We found the following java versions on your pc:",
                      }}
                    />
                  </h2>
                  <Button
                    rounded={false}
                    variant="secondary"
                    size="small"
                    onClick={() => {
                      modalsContext?.openModal({ name: "addJava" });
                    }}
                  >
                    <div class="text-darkSlate-500 text-xl i-ri:add-fill" />
                  </Button>
                </div>
                <For each={Object.entries(javas())}>
                  {([javaVersion, obj]) => (
                    <div class="rounded-xl border-1 border-solid border-darkSlate-600 p-4">
                      <h3 class="m-0 mb-4">{javaVersion}</h3>
                      <div class="flex flex-col gap-4">
                        <For each={obj}>
                          {(java) => (
                            <div class="rounded-lg border-1 border-solid border-darkSlate-600 px-4 flex justify-between items-center py-2 bg-darkSlate-700">
                              <span class="text-sm">{java.path}</span>
                              <span>{java.type}</span>
                            </div>
                          )}
                        </For>
                      </div>
                    </div>
                  )}
                </For>
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
      </div>
    </div>
  );
};

export default Java;
