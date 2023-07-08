import { rspc } from "@/utils/rspcClient";
import { Trans } from "@gd/i18n";
import { Button, Checkbox, Tag } from "@gd/ui";
import {
  For,
  Match,
  Switch,
  createEffect,
  createSignal,
  onMount,
} from "solid-js";

const Import = () => {
  const scanImportableInstancesMutation = rspc.createMutation([
    "instance.scanImportableInstances",
  ]);

  const importInstanceMutation = rspc.createMutation([
    "instance.importInstance",
  ]);

  const instances = rspc.createQuery(() => [
    "instance.getImportableInstances",
    "LegacyGDLauncher",
  ]);

  onMount(() => {
    scanImportableInstancesMutation.mutate("LegacyGDLauncher");
  });

  const [selectedInstancesIds, setSelectedInstancesIds] = createSignal([]);
  const [selectedEntity, setSelectedEntity] = createSignal("LegacyGDLauncher");

  const entities = ["LegacyGDLauncher", "Curseforge", "Modrinth"];

  return (
    <div class="p-5 h-full flex flex-col justify-between items-end box-border">
      <div class="flex flex-col gap-4 w-full">
        <div class="flex gap-4 w-full">
          <For each={entities}>
            {(entity) => (
              <div
                class="px-3 py-2 bg-darkSlate-800 rounded-lg cursor-pointer border-box"
                classList={{
                  "border-2 border-solid border-transparent":
                    selectedEntity() !== entity,
                  "border-2 border-solid border-primary-500":
                    selectedEntity() === entity,
                }}
                onClick={() => setSelectedEntity(entity)}
              >
                {entity}
              </div>
            )}
          </For>
        </div>
        <div class="w-full h-50 bg-darkSlate-800 rounded-xl box-border flex flex-col overflow-hidden">
          <div class="flex justify-between w-full bg-darkSlate-900 px-4 py-2 box-border">
            <Checkbox onChange={(checked) => {}} />
            <span class="cursor-pointer">
              <Trans key="instance.import_select_all_instances" />
            </span>
          </div>
          <div class="p-4 h-full w-full box-border flex flex-col gap-4">
            <For each={instances.data}>
              {(instance) => (
                <div class="flex gap-2 w-full">
                  <Checkbox
                    onChange={(checked) => {
                      // if(checked) setSelectedInstancesIds(prev => [...prev, instance.])
                    }}
                  />
                  <span>{instance.name}</span>
                </div>
              )}
            </For>
          </div>
        </div>
      </div>
      <Button
        onClick={() => {
          importInstanceMutation.mutate({
            entity: "LegacyGDLauncher",
            index: 0,
          });
        }}
      >
        <div class="i-ri:folder-open-fill" />
        <Switch>
          <Match when={selectedInstancesIds().length > 0}>
            <Trans
              key="instance.import_instance_amount"
              options={{
                instances_amount: selectedInstancesIds().length,
              }}
            />
          </Match>
          <Match when={selectedInstancesIds().length === 0}>
            <Trans
              key="instance.import_instance"
              options={{
                instances_amount: selectedInstancesIds().length,
              }}
            />
          </Match>
        </Switch>
      </Button>
    </div>
  );
};

export default Import;
