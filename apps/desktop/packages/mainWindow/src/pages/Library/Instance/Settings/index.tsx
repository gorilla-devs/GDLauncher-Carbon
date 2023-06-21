/* eslint-disable i18next/no-literal-string */
import { generateSequence } from "@/utils/helpers";
import { queryClient, rspc } from "@/utils/rspcClient";
import { Trans } from "@gd/i18n";
import { Button, Input, Slider, Switch } from "@gd/ui";
import { useParams, useRouteData } from "@solidjs/router";
import fetchData from "../instance.data";
import { Show } from "solid-js";

const Settings = () => {
  const params = useParams();
  const updateInstanceMutation = rspc.createMutation(
    ["instance.updateInstance"],
    {
      onMutate: (newData) => {
        queryClient.setQueryData(["instance.getInstanceDetails"], newData);
      },
    }
  );

  let totalRam = rspc.createQuery(() => ["systeminfo.getTotalRAM"]);

  const routeData: ReturnType<typeof fetchData> = useRouteData();

  const mbTotalRAM = () => Number(totalRam.data) / 1024 / 1024;

  return (
    <div class="pt-10">
      <div class="mb-6">
        <div class="w-full flex justify-between items-center mb-4">
          <h5 class="m-0">
            <Trans
              key="java.java_memory_title"
              options={{
                defaultValue: "Java Memory",
              }}
            />
          </h5>
          <Switch
            checked={!!routeData?.instanceDetails?.data?.memory}
            onChange={(e) => {
              updateInstanceMutation.mutate({
                memory: {
                  Set: e.target.checked
                    ? { max_mb: mbTotalRAM() / 2, min_mb: mbTotalRAM() / 2 }
                    : null,
                },
                extra_java_args: null,
                global_java_args: null,
                modloader: null,
                name: null,
                notes: null,
                use_loaded_icon: null,
                version: null,
                instance: parseInt(params.id, 10),
              });
            }}
          />
        </div>
        <Show when={routeData?.instanceDetails?.data?.memory !== null}>
          <div class="flex justify-center px-2">
            <Slider
              min={0}
              max={mbTotalRAM()}
              steps={1000}
              value={routeData?.instanceDetails.data?.memory?.max_mb}
              marks={generateSequence(1024, mbTotalRAM())}
              onChange={(val) =>
                updateInstanceMutation.mutate({
                  memory: { Set: { max_mb: val, min_mb: val } },
                  extra_java_args: null,
                  global_java_args: null,
                  modloader: null,
                  name: null,
                  notes: null,
                  use_loaded_icon: null,
                  version: null,
                  instance: parseInt(params.id, 10),
                })
              }
            />
          </div>
        </Show>
      </div>
      {/* <div class="mb-6">
        <div class="w-full flex justify-between items-center mb-4">
          <h5 class="m-0">
            <Trans
              key="java.game_resolution"
              options={{
                defaultValue: "Game Resolution",
              }}
            />
          </h5>
          <Switch checked={true} />
        </div>
        <div class="flex w-full justify-between">
          <div class="flex gap-4 items-center">
            <Input class="w-20" placeholder="width" value={"1024"} />
            x
            <Input class="w-20" placeholder="height" value={"768"} />
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
      </div> */}
      <div class="mb-6">
        <div class="w-full flex justify-between items-center mb-4">
          <h5 class="m-0">
            <Trans
              key="java.java_arguments_title"
              options={{
                defaultValue: "Java Arguments",
              }}
            />
          </h5>
          <Switch
            checked={routeData?.instanceDetails?.data?.global_java_args}
            onChange={(e) => {
              updateInstanceMutation.mutate({
                memory: null,
                extra_java_args: null,
                global_java_args: { Set: e.target.checked },
                modloader: null,
                name: null,
                notes: null,
                use_loaded_icon: null,
                version: null,
                instance: parseInt(params.id, 10),
              });
            }}
          />
        </div>
        <Show when={routeData?.instanceDetails?.data?.global_java_args}>
          <div class="flex w-full gap-4 items-center">
            <Input
              class="w-full"
              value={routeData?.instanceDetails?.data?.extra_java_args || ""}
              onInput={(e) => {
                updateInstanceMutation.mutate({
                  memory: null,
                  extra_java_args: { Set: e.target.value },
                  global_java_args: null,
                  modloader: null,
                  name: null,
                  notes: null,
                  use_loaded_icon: null,
                  version: null,
                  instance: parseInt(params.id, 10),
                });
              }}
            />
            <Button
              rounded={false}
              type="secondary"
              class="h-10"
              onClick={() => {
                updateInstanceMutation.mutate({
                  memory: null,
                  extra_java_args: { Set: "" },
                  global_java_args: null,
                  modloader: null,
                  name: null,
                  notes: null,
                  use_loaded_icon: null,
                  version: null,
                  instance: parseInt(params.id, 10),
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
        </Show>
      </div>
    </div>
  );
};

export default Settings;
