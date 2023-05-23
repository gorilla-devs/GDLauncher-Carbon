import { Trans } from "@gd/i18n";
import {
  Button,
  Input,
  Slider,
  Switch,
  Tab,
  TabList,
  TabPanel,
  Tabs,
} from "@gd/ui";
import { useRouteData } from "@solidjs/router";
import { For, createEffect, createSignal } from "solid-js";
import SettingsJavaData from "./settings.java.data";
import { useModal } from "@/managers/ModalsManager";

const Java = () => {
  const routeData: ReturnType<typeof SettingsJavaData> = useRouteData();
  const javasData = () => routeData?.availableJavas;
  const javas = () => javasData()?.data || [];
  const modalsContext = useModal();

  // let setDefaultJavaMutation = rspc.createMutation(["java.setDefault"], {
  //   onMutate: (newTheme) => {
  //     queryClient.setQueryData(["java.setDefault", null], newTheme);
  //   },
  // });

  createEffect(() => {
    console.log("JAVAS", javas(), routeData.javaProfiles.data);
  });

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

      <div class="mb-6">
        <h5 class="m-0 mb-4">
          <Trans
            key="java.java_memory_title"
            options={{
              defaultValue: "Java Memory",
            }}
          />
        </h5>
        <div class="flex justify-center">
          <Slider
            min={0}
            max={16384}
            steps={1}
            marks={{
              1024: "1024 MB",
              2048: "2048 MB",
              4096: "4096 MB",
              8192: "8192 MB",
              16384: "16384 MB",
            }}
          />
        </div>
      </div>
      <div class="mb-6">
        <h5 class="m-0 mb-4">
          <Trans
            key="java.java_arguments_title"
            options={{
              defaultValue: "Java Arguments",
            }}
          />
        </h5>
        <div class="flex w-full gap-4 items-center">
          <Input class="w-full" />
          <Button rounded={false} variant="secondary" class="h-10">
            <Trans
              key="java.reset_java_args"
              options={{
                defaultValue: "Reset",
              }}
            />
          </Button>
        </div>
      </div>
      <div class="h-full flex justify-between mb-6 mt-10">
        <h2 class="mt-0 text-sm">
          <Trans
            key="java.auto_handle_java"
            options={{
              defaultValue: "Auto handle java",
            }}
          />
        </h2>
        <Switch />
      </div>
      <div class="flex flex-col">
        <div class="flex justify-between mb-4">
          <h5 class="m-0 flex items-center">
            <Trans
              key="java.all_versions"
              options={{
                defaultValue: "All versions",
              }}
            />
          </h5>
          <Button
            rounded={false}
            variant="secondary"
            onClick={() => {
              modalsContext?.openModal({ name: "addJava" });
            }}
          >
            <div class="text-darkSlate-500 text-xl i-ri:add-fill" />
          </Button>
        </div>
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
            <div class="bg-darkSlate-900 h-full p-4">
              <h2 class="mt-0 mb-6 text-sm">
                <Trans
                  key="java.found_java_text"
                  options={{
                    defaultValue:
                      "We found the following java versions on your pc:",
                  }}
                />
              </h2>
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
            <div class="bg-darkSlate-900 h-full p-4 flex flex-col gap-4">
              <For each={routeData.javaProfiles.data}>
                {(profile) => (
                  <div class="rounded-xl border-1 border-solid border-darkSlate-600 p-4">
                    <h3 class="m-0">{profile.name}</h3>
                  </div>
                )}
              </For>
            </div>
          </TabPanel>
        </Tabs>
      </div>
    </div>
  );
};

export default Java;
