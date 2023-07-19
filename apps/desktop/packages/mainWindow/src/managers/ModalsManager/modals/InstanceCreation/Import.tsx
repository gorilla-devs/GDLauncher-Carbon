import {
  importedInstances,
  isProgressFailed,
  setImportedInstances,
} from "@/utils/instances";
import { rspc } from "@/utils/rspcClient";
import {
  FEEntity,
  FEImportableInstance,
  FETask,
} from "@gd/core_module/bindings";
import { Trans } from "@gd/i18n";
import { Button, Checkbox, Spinner } from "@gd/ui";
import { RSPCError } from "@rspc/client";
import { CreateQueryResult } from "@tanstack/solid-query";
import { For, Match, Show, Switch, createEffect, createSignal } from "solid-js";
import { createStore } from "solid-js/store";

const Import = () => {
  const [selectedEntity, setSelectedEntity] =
    createSignal<FEEntity>("legacyGDLauncher");
  const [isLoading, setIsLoading] = createSignal(false);
  const [selectedInstancesIds, setSelectedInstancesIds] = createStore<{
    [id: number]: boolean;
  }>({});
  const [loadingInstances, setLoadingInstances] = createStore<{
    [id: number]: FETask | null | undefined;
  }>({});
  const [instances, setInstances] =
    createSignal<CreateQueryResult<FEImportableInstance[], RSPCError>>();
  const [taskId, setTaskId] = createSignal<undefined | number>(undefined);
  const [currentInstanceIndex, setCurrentInstanceIndex] = createSignal(0);

  const scanImportableInstancesMutation = rspc.createMutation([
    "instance.scanImportableInstances",
  ]);

  const entries = () => Object.entries(selectedInstancesIds);
  const filteredEntries = () =>
    entries().filter(([_i, isSelected]) => isSelected);

  const importInstanceMutation = rspc.createMutation(
    ["instance.importInstance"],
    {
      onSuccess(taskId, entity) {
        console.log("ONSUCCESS", taskId, entity.index);
        setTaskId(taskId);
      },
    }
  );

  createEffect(() => {
    if (taskId() !== undefined) {
      console.log("TASK ID", taskId());

      const task = rspc.createQuery(() => [
        "vtask.getTask",
        taskId() as number,
      ]);

      const isFailed = task.data && isProgressFailed(task.data.progress);
      const isDownloaded =
        task.data &&
        task.data?.download_total > 0 &&
        task.data?.downloaded === task.data?.download_total;

      const nextInstance = filteredEntries()[currentInstanceIndex()];
      if (!nextInstance) return;
      const instanceIndex = parseInt(nextInstance[0], 10);
      console.log(
        "TASK",
        instanceIndex,
        task.data,
        (isDownloaded || isFailed) && !task.isLoading,
        task.data && !task.isInitialLoading
      );

      setLoadingInstances(instanceIndex, task.data);

      if (isDownloaded || isFailed) {
        setLoadingInstances(instanceIndex, undefined);

        console.log("NEXT");
        setImportedInstances((prev) => [...prev, instanceIndex]);

        importInstanceMutation.mutate({
          entity: selectedEntity(),
          index: instanceIndex,
        });

        setCurrentInstanceIndex((prev) => prev + 1);
      }
    }
  });

  createEffect(() => {
    setInstances(
      // eslint-disable-next-line solid/reactivity
      rspc.createQuery(() => [
        "instance.getImportableInstances",
        selectedEntity(),
      ])
    );
  });

  const entities = rspc.createQuery(() => ["instance.getImportableEntities"]);

  const currentlySupportedEnties = ["legacyGDLauncher"];

  createEffect(() => {
    scanImportableInstancesMutation.mutate(selectedEntity());
  });

  const selectAll = (checked: boolean) => {
    instances()?.data?.forEach((_instance, i) => {
      setSelectedInstancesIds((prev) => ({
        ...prev,
        [i]: checked,
      }));
    });
  };

  const areAllSelected = () =>
    Object.values(selectedInstancesIds).filter((instance) => instance)
      .length === instances()?.data?.length;

  const isAllImported = () =>
    importedInstances().length === instances()?.data?.length;

  createEffect(() => {
    if (isAllImported()) setIsLoading(false);
  });

  return (
    <div class="p-5 h-full flex flex-col justify-between box-border items-end overflow-x-hidden">
      <div class="flex flex-col gap-4 w-full">
        <div class="overflow-x-auto w-fill">
          <div class="flex gap-4 py-2">
            <For each={entities.data}>
              {(entity) => (
                <div
                  class="relative flex justify-center px-3 py-2 bg-darkSlate-800 rounded-lg cursor-pointer min-w-30"
                  classList={{
                    "border-2 border-solid border-transparent text-darkSlate-400 cursor-not-allowed":
                      !currentlySupportedEnties.includes(entity) ||
                      selectedEntity() !== entity,
                    "border-2 border-solid border-primary-500":
                      currentlySupportedEnties.includes(entity) ||
                      selectedEntity() === entity,
                  }}
                  onClick={() => {
                    if (currentlySupportedEnties.includes(entity))
                      setSelectedEntity(entity);
                  }}
                >
                  <Show when={entity !== "legacyGDLauncher"}>
                    <span class="absolute rounded-full text-white text-xs -top-2 -right-4 bg-green-500 px-1">
                      <Trans key="instances.import_entity_coming_soon" />
                    </span>
                  </Show>
                  {entity}
                </div>
              )}
            </For>
          </div>
        </div>
        <div class="w-full bg-darkSlate-800 rounded-xl box-border flex flex-col overflow-hidden h-50">
          <div class="flex justify-between w-full bg-darkSlate-900 px-4 py-2 box-border">
            <Checkbox
              disabled={isAllImported()}
              checked={areAllSelected()}
              onChange={(checked) => {
                selectAll(checked);
              }}
            />
            <span
              class="cursor-pointer"
              classList={{
                "text-darkSlate-600": isAllImported(),
              }}
              onClick={() => {
                if (!isAllImported()) selectAll(!areAllSelected());
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
            <Match when={(instances()?.data?.length || 0) > 0}>
              <div class="p-4 h-full w-full box-border flex flex-col gap-4">
                <For each={instances()?.data}>
                  {(instance, i) => (
                    <div class="flex justify-between">
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
                      <Show
                        when={
                          loadingInstances[i()]

                          // loadingInstances[i()] &&
                          // loadingInstances[i()]?.downloaded <
                          //   loadingInstances[i()]?.download_total
                        }
                      >
                        <Spinner />
                      </Show>
                    </div>
                  )}
                </For>
              </div>
            </Match>
            <Match when={(instances()?.data?.length || 0) === 0}>
              <div class="p-4 h-full w-full box-border flex flex-col justify-center items-center">
                <Trans key="instance.import_no_instances" />
              </div>
            </Match>
          </Switch>
        </div>
      </div>
      <Button
        disabled={importedInstances().length === instances()?.data?.length}
        onClick={() => {
          setIsLoading(true);
          const indexFirstElement = parseInt(filteredEntries()[0][0], 10);
          console.log("entries()", entries());
          importInstanceMutation.mutate({
            entity: selectedEntity(),
            index: indexFirstElement,
          });
        }}
      >
        <Switch>
          <Match
            when={
              Object.values(selectedInstancesIds).filter((id) => id).length >
                0 && !isLoading()
            }
          >
            <div class="i-ri:folder-open-fill" />
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
                0 && !isLoading()
            }
          >
            <div class="i-ri:folder-open-fill" />
            <Trans key="instance.import_instance" />
          </Match>
          <Match when={isLoading()}>
            <Spinner />
          </Match>
        </Switch>
      </Button>
    </div>
  );
};

export default Import;
