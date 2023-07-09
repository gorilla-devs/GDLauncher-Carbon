import { rspc } from "@/utils/rspcClient";
import { FEEntity, FEImportInstance } from "@gd/core_module/bindings";
import { Trans } from "@gd/i18n";
import { Button, Checkbox } from "@gd/ui";
import { RSPCError } from "@rspc/client";
import { CreateMutationResult } from "@tanstack/solid-query";
import { For, Match, Show, Switch, createSignal, onMount } from "solid-js";
import { createStore } from "solid-js/store";

const Import = () => {
  const [selectedEntity, setSelectedEntity] =
    createSignal<FEEntity>("legacyGDLauncher");
  const [isLoading, setIsLoading] = createSignal(false);
  const [selectedInstancesIds, setSelectedInstancesIds] = createStore<{
    [id: number]: boolean;
  }>({});
  const [importedInstances, setImportedInstances] = createSignal<number[]>([]);

  const scanImportableInstancesMutation = rspc.createMutation([
    "instance.scanImportableInstances",
  ]);

  const importInstanceMutation = rspc.createMutation(
    ["instance.importInstance"],
    {
      onSuccess(_data, entity) {
        setImportedInstances((prev) => [...prev, entity.index]);
      },
    }
  );

  const instances = rspc.createQuery(() => [
    "instance.getImportableInstances",
    selectedEntity(),
  ]);

  const entities = rspc.createQuery(() => ["instance.getImportableEntities"]);

  const currentlySupportedEnties = ["legacyGDLauncher"];

  onMount(() => {
    scanImportableInstancesMutation.mutate("legacyGDLauncher");
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

  async function processEntries(
    entries: [string, boolean][],
    importInstanceMutation: CreateMutationResult<
      null,
      RSPCError,
      FEImportInstance,
      unknown
    >,
    selectedEntity: any
  ) {
    for (let index = 0; index < entries.length; index++) {
      const [i, boolean] = entries[index];

      if (boolean) {
        await importInstanceMutation.mutate({
          entity: selectedEntity(),
          index: parseInt(i, 10),
        });
      }
    }
  }

  return (
    <div class="p-5 h-full flex flex-col justify-between items-end box-border overflow-x-hidden">
      <div class="flex flex-col gap-4 w-full">
        <div class="w-fill scrollbar-hide overflow-x-auto">
          <div class="flex gap-4 py-2">
            <For each={entities.data}>
              {(entity) => (
                <div
                  class="relative flex justify-center px-3 py-2 min-w-30 bg-darkSlate-800 rounded-lg cursor-pointer"
                  // classList={{
                  //   "border-2 border-solid border-transparent":
                  //     selectedEntity() !== entity,
                  //   "border-2 border-solid border-primary-500":
                  //     selectedEntity() === entity,
                  // }}
                  classList={{
                    "border-2 border-solid border-transparent text-darkSlate-400 cursor-not-allowed":
                      !currentlySupportedEnties.includes(entity),
                    "border-2 border-solid border-primary-500":
                      currentlySupportedEnties.includes(entity),
                  }}
                  // classList={{
                  //   "border-2 border-solid border-transparent":
                  //     selectedEntity() !== entity,
                  //   "border-2 border-solid border-primary-500":
                  //     selectedEntity() === entity,
                  // }}
                  // onClick={() => setSelectedEntity(entity)}
                >
                  <Show when={entity !== "legacyGDLauncher"}>
                    <span class="absolute -top-2 -right-4 bg-green-500 rounded-full text-white text-xs px-1">
                      Coming soon
                    </span>
                  </Show>
                  {entity}
                </div>
              )}
            </For>
          </div>
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
          <Switch>
            <Match when={(instances.data?.length || 0) > 0}>
              <div class="p-4 h-full w-full box-border flex flex-col gap-4">
                <For each={instances.data}>
                  {(instance, i) => (
                    <div class="flex gap-2 w-full">
                      <Checkbox
                        disabled={importedInstances().includes(i())}
                        checked={selectedInstancesIds[i()]}
                        onChange={(checked) => {
                          setSelectedInstancesIds((prev) => ({
                            ...prev,
                            [i()]: checked,
                          }));
                        }}
                      />
                      <span
                        classList={{
                          "text-darkSlate-500": importedInstances().includes(
                            i()
                          ),
                        }}
                      >
                        {instance.name}
                      </span>
                    </div>
                  )}
                </For>
              </div>
            </Match>
            <Match when={(instances.data?.length || 0) === 0}>
              <div class="p-4 h-full w-full box-border flex flex-col justify-center items-center">
                <Trans key="instance.import_no_instances" />
              </div>
            </Match>
          </Switch>
        </div>
      </div>
      <Button
        disabled={
          isLoading() || importedInstances().length === instances.data?.length
        }
        onClick={() => {
          const entries = Object.entries(selectedInstancesIds);
          setIsLoading(true);
          processEntries(entries, importInstanceMutation, selectedEntity);
          setIsLoading(false);
        }}
      >
        <div class="i-ri:folder-open-fill" />
        <Switch>
          <Match
            when={
              Object.values(selectedInstancesIds).filter((id) => id).length >
                0 && !isLoading()
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
