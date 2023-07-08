import { rspc } from "@/utils/rspcClient";
import { FEEntity } from "@gd/core_module/bindings";
import { Trans } from "@gd/i18n";
import { Button, Checkbox } from "@gd/ui";
import { For, Match, Show, Switch, createSignal, onMount } from "solid-js";
import { createStore } from "solid-js/store";

const Import = () => {
  const [selectedEntity, setSelectedEntity] =
    createSignal<FEEntity>("LegacyGDLauncher");
  const [selectedInstancesIds, setSelectedInstancesIds] = createStore<{
    [id: number]: boolean;
  }>({});

  const scanImportableInstancesMutation = rspc.createMutation([
    "instance.scanImportableInstances",
  ]);

  const importInstanceMutation = rspc.createMutation([
    "instance.importInstance",
  ]);

  const instances = rspc.createQuery(() => [
    "instance.getImportableInstances",
    selectedEntity(),
  ]);

  //   const entities = rspc.createQuery(() => ["instance.getImportableEntities"]);

  const entities = ["LegacyGDLauncher", "Curseforge", "Modrinth"];

  onMount(() => {
    scanImportableInstancesMutation.mutate("LegacyGDLauncher");
  });

  const selectAll = (checked: boolean) => {
    instances.data?.forEach((_instance, i) => {
      setSelectedInstancesIds((prev) => ({
        ...prev,
        [i]: checked,
      }));
    });
  };

  const areAllSelected = () =>
    Object.values(selectedInstancesIds).filter((instance) => instance)
      .length === instances.data?.length;

  return (
    <div class="p-5 h-full flex flex-col justify-between items-end box-border">
      <div class="flex flex-col gap-4 w-full">
        <div class="flex gap-4 w-full">
          <For each={entities}>
            {(entity) => (
              <div
                class="relative px-3 py-2 bg-darkSlate-800 rounded-lg cursor-pointer border-box"
                // classList={{
                //   "border-2 border-solid border-transparent":
                //     selectedEntity() !== entity,
                //   "border-2 border-solid border-primary-500":
                //     selectedEntity() === entity,
                // }}
                classList={{
                  "border-2 border-solid border-transparent text-darkSlate-400 cursor-not-allowed":
                    entity !== "LegacyGDLauncher",
                  "border-2 border-solid border-primary-500":
                    entity === "LegacyGDLauncher",
                }}
                // classList={{
                //   "border-2 border-solid border-transparent":
                //     selectedEntity() !== entity,
                //   "border-2 border-solid border-primary-500":
                //     selectedEntity() === entity,
                // }}
                // onClick={() => setSelectedEntity(entity)}
              >
                <Show when={entity !== "LegacyGDLauncher"}>
                  <span class="absolute -top-2 -right-4 bg-green-500 rounded-full text-white text-xs px-1">
                    Coming soon
                  </span>
                </Show>
                {entity}
              </div>
            )}
          </For>
        </div>
        <div class="w-full h-50 bg-darkSlate-800 rounded-xl box-border flex flex-col overflow-hidden">
          <div class="flex justify-between w-full bg-darkSlate-900 px-4 py-2 box-border">
            <Checkbox
              checked={areAllSelected()}
              onChange={(checked) => {
                selectAll(checked);
              }}
            />
            <span
              class="cursor-pointer"
              onClick={() => {
                selectAll(!areAllSelected());
              }}
            >
              <Switch>
                <Match when={!areAllSelected()}>
                  <Trans key="instance.import_select_all_instances" />
                </Match>
                <Match when={areAllSelected()}>
                  <Trans key="instance.import_deselect_all_instances" />
                </Match>
              </Switch>
            </span>
          </div>
          <div class="p-4 h-full w-full box-border flex flex-col gap-4">
            <For each={instances.data}>
              {(instance, i) => (
                <div class="flex gap-2 w-full">
                  <Checkbox
                    checked={selectedInstancesIds[i()]}
                    onChange={(checked) => {
                      setSelectedInstancesIds((prev) => ({
                        ...prev,
                        [i()]: checked,
                      }));
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
          Object.entries(selectedInstancesIds).forEach(([index, boolean]) => {
            if (boolean) {
              const loading = importInstanceMutation.mutate({
                entity: selectedEntity(),
                index: parseInt(index, 10),
              });
            }
          });
        }}
      >
        <div class="i-ri:folder-open-fill" />
        <Switch>
          <Match
            when={
              Object.values(selectedInstancesIds).filter((id) => id).length > 0
            }
          >
            <Trans
              key="instance.import_instance_amount"
              options={{
                instances_amount: Object.values(selectedInstancesIds).filter(
                  (id) => id
                ).length,
              }}
            />
          </Match>
          <Match
            when={
              Object.values(selectedInstancesIds).filter((id) => id).length ===
              0
            }
          >
            <Trans key="instance.import_instance" />
          </Match>
        </Switch>
      </Button>
    </div>
  );
};

export default Import;
