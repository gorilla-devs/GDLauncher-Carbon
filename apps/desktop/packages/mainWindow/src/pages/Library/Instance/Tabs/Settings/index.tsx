import { generateSequence } from "@/utils/helpers";
import { queryClient, rspc } from "@/utils/rspcClient";
import { Trans } from "@gd/i18n";
import { Button, Input, Slider, Switch } from "@gd/ui";
import { useParams, useRouteData } from "@solidjs/router";
import fetchData from "../../instance.data";
import { Show, Suspense, createMemo } from "solid-js";
import { InstanceDetails } from "@gd/core_module/bindings";
import Title from "@/pages/Settings/components/Title";
import Row from "@/pages/Settings/components/Row";
import RowsContainer from "@/pages/Settings/components/RowsContainer";
import RightHandSide from "@/pages/Settings/components/RightHandSide";

const Settings = () => {
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

    return routeData?.instanceDetails?.data?.extra_java_args as string | null;
  }, null);

  const mbTotalRAM = () => Number(routeData.totalRam.data) / 1024 / 1024;

  return (
    <Suspense fallback={null}>
      <RowsContainer>
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
                  extra_java_args: null,
                  global_java_args: null,
                  modloader: null,
                  name: null,
                  notes: null,
                  use_loaded_icon: null,
                  version: null,
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
                  extra_java_args: null,
                  global_java_args: null,
                  modloader: null,
                  name: null,
                  notes: null,
                  use_loaded_icon: null,
                  version: null,
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
              routeData?.instanceDetails?.data?.extra_java_args !== null &&
              routeData?.instanceDetails?.data?.extra_java_args !== undefined
            }
            onChange={(e) => {
              const checked = e.target.checked;

              updateInstanceMutation.mutate({
                memory: null,
                extra_java_args: { Set: checked ? "" : null },
                global_java_args: null,
                modloader: null,
                name: null,
                notes: null,
                use_loaded_icon: null,
                version: null,
                instance: parseInt(params.id, 10)
              });
            }}
          />
        </Row>
        <Show
          when={
            routeData?.instanceDetails?.data?.extra_java_args !== null &&
            routeData?.instanceDetails?.data?.extra_java_args !== undefined
          }
        >
          <div class="flex w-full justify-between items-center -mt-8">
            <h5 class="text-lightSlate-800">
              <Trans key="instance_settings.prepend_global_java_args" />
            </h5>
            <Switch
              checked={routeData?.instanceDetails?.data?.global_java_args}
              onChange={(e) => {
                const checked = e.target.checked;

                updateInstanceMutation.mutate({
                  memory: null,
                  extra_java_args: null,
                  global_java_args: { Set: checked },
                  modloader: null,
                  name: null,
                  notes: null,
                  use_loaded_icon: null,
                  version: null,
                  instance: parseInt(params.id, 10)
                });
              }}
            />
          </div>
          <div class="flex w-full gap-4 items-center">
            <Show when={routeData?.instanceDetails?.data?.global_java_args}>
              <Input
                class="w-1/3"
                inputClass="font-bold"
                disabled
                value="{GLOBAL_JAVA_ARGS}"
              />
              <div>+</div>
            </Show>
            <Input
              class="w-full"
              value={routeData?.instanceDetails?.data?.extra_java_args || ""}
              onChange={(e) => {
                updateInstanceMutation.mutate({
                  memory: null,
                  extra_java_args: { Set: e.target.value },
                  global_java_args: null,
                  modloader: null,
                  name: null,
                  notes: null,
                  use_loaded_icon: null,
                  version: null,
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
                  memory: null,
                  extra_java_args: { Set: initialJavaArgs() },
                  global_java_args: null,
                  modloader: null,
                  name: null,
                  notes: null,
                  use_loaded_icon: null,
                  version: null,
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
                  memory: null,
                  extra_java_args: { Set: "" },
                  global_java_args: null,
                  modloader: null,
                  name: null,
                  notes: null,
                  use_loaded_icon: null,
                  version: null,
                  instance: parseInt(params.id, 10)
                });
              }}
            >
              <i class="w-5 h-5 i-ri:close-fill" />
            </Button>
          </div>
        </Show>
      </RowsContainer>
    </Suspense>
  );
};

export default Settings;
