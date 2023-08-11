import {
  currentInstanceIndex,
  instances,
  loadingInstances,
  selectedInstancesIndexes,
  setCurrentInstanceIndex,
  setInstances,
  setLoadingInstances,
  setSelectedInstanceNames,
  setSelectedInstancesIndexes,
  setTaskId,
  taskId,
} from "@/utils/import";
import {
  importedInstances,
  isProgressFailed,
  isProgressKnown,
  setImportedInstances,
} from "@/utils/instances";
import { rspc } from "@/utils/rspcClient";
import { FEEntity, FETask } from "@gd/core_module/bindings";
import { Trans, useTransContext } from "@gd/i18n";
import { Button, Checkbox, Input, Spinner } from "@gd/ui";
import {
  For,
  Match,
  Setter,
  Show,
  Switch,
  createEffect,
  createSignal,
} from "solid-js";

import { OpenDialogOptions, dialog } from "electron";

type Props = {
  setIsLoading?: Setter<boolean>;
};

const Import = (props: Props) => {
  const [selectedEntity, setSelectedEntity] =
    createSignal<FEEntity>("legacygdlauncher");
  const [isLoading, setIsLoading] = createSignal(false);

  const [t] = useTransContext();

  const scanImportableInstancesMutation = rspc.createMutation([
    "instance.scanImportableInstances",
  ]);

  const entries = () => Object.entries(selectedInstancesIndexes);
  const selectedEntires = () =>
    entries().filter(([_i, isSelected]) => isSelected);

  const importInstanceMutation = rspc.createMutation(
    ["instance.importInstance"],
    {
      onSuccess(taskId) {
        setTaskId(taskId);
      },
    }
  );

  let currentIndex = currentInstanceIndex();
  createEffect(() => {
    if (taskId() !== undefined) {
      // eslint-disable-next-line solid/reactivity
      const task = rspc.createQuery(() => [
        "vtask.getTask",
        taskId() as number,
      ]);

      const isFailed = task.data && isProgressFailed(task.data.progress);
      const isDownloaded = task.data === null;

      const currentInstance = selectedEntires()[currentIndex];
      if (!currentInstance) return;
      const instanceIndex = parseInt(currentInstance[0], 10);

      setLoadingInstances(instanceIndex, task.data);

      if (isDownloaded || isFailed) {
        currentIndex = setCurrentInstanceIndex((prev) => prev + 1);
        setLoadingInstances(instanceIndex, null);
        setImportedInstances((prev) => [...prev, instanceIndex]);

        const nextInstance = selectedEntires()[currentIndex];
        if (!nextInstance) return;
        const nextInstanceIndex = parseInt(nextInstance[0], 10);

        importInstanceMutation.mutate({
          entity: selectedEntity(),
          index: nextInstanceIndex,
          name: "", // TODO: edit name
        });
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

  const currentlySupportedEntities = [
    "legacygdlauncher",
    "mrpack",
    "curseforgezip",
  ];

  createEffect(() => {
    scanImportableInstancesMutation.mutate(selectedEntity());
  });

  const selectAll = (checked: boolean) => {
    instances()?.data?.forEach((_instance, i) => {
      setSelectedInstancesIndexes((prev) => ({
        ...prev,
        [i]: checked,
      }));
    });
  };

  const areAllSelected = () =>
    Object.values(selectedInstancesIndexes).filter((instance) => instance)
      .length === instances()?.data?.length;

  const isAllImported = () =>
    importedInstances().length === instances()?.data?.length;

  createEffect(() => {
    if (isAllImported()) {
      setIsLoading(false);
      props?.setIsLoading?.(false);
    }
  });

  const isMRPack = (entity: FEEntity): entity is { mrpack: string } => {
    if (typeof entity === "string") {
      return false;
    } else {
      return "mrpack" in entity;
    }
  };

  const isCurseforgeZip = (
    entity: FEEntity
  ): entity is { curseforgezip: string } => {
    if (typeof entity === "string") {
      return false;
    } else {
      return "curseforgezip" in entity;
    }
  };

  const nameFromEntity = (entity: FEEntity): string => {
    if (isCurseforgeZip(entity)) {
      return "curseforgezip";
    } else if (isMRPack(entity)) {
      return "mrpack";
    } else {
      return entity;
    }
  };

  const dialogPropertiesFromEntity = (entity: FEEntity): OpenDialogOptions => {
    if (isMRPack(entity)) {
      return {
        title: t("instance.import_select_mrpack") as string,
        properties: ["openFile"],
        filters: [{ name: "Mrpack", extensions: ["mrpack"] }],
      };
    } else if (isCurseforgeZip(entity)) {
      return {
        title: t("instance.import_select_curseforgezip") as string,
        properties: ["openFile"],
        filters: [{ name: "Curseforge Zip", extensions: ["zip"] }],
      };
    } else {
      return {
        title: t("instance.import_select_scan_path") as string,
        properties: ["openDirectory"],
        filters: [],
      };
    }
  };

  let inputRef: HTMLInputElement | undefined;
  const [scanPath, setScanPath] = createSignal("");

  return (
    <div class="p-5 h-full flex flex-col justify-between box-border items-end overflow-x-hidden">
      <div class="flex flex-col gap-4 w-full">
        <div class="overflow-x-auto w-fill">
          <div class="flex gap-4 py-2">
            <For each={entities.data}>
              {(entity) => {
                const entityName = nameFromEntity(entity);
                return (
                  <div
                    class="relative flex justify-center px-4 py-2 bg-darkSlate-800 rounded-lg cursor-pointer whitespace-nowrap min-w-30"
                    classList={{
                      "border-2 border-solid border-transparent text-darkSlate-200":
                        selectedEntity() !== entity,
                      "border-2 border-solid border-transparent text-darkSlate-400 cursor-not-allowed":
                        !currentlySupportedEntities.includes(entityName),
                      "border-2 border-solid border-primary-500":
                        currentlySupportedEntities.includes(entityName) ||
                        selectedEntity() === entity,
                    }}
                    onClick={() => {
                      if (currentlySupportedEntities.includes(entityName))
                        setSelectedEntity(entity);
                    }}
                  >
                    <Show
                      when={!currentlySupportedEntities.includes(entityName)}
                    >
                      <span class="absolute rounded-full text-white text-xs -top-2 -right-4 bg-green-500 px-1">
                        <Trans key="instances.import_entity_coming_soon" />
                      </span>
                    </Show>
                    {t(`entity.${entityName}`)}
                  </div>
                );
              }}
            </For>
          </div>
        </div>
        <div class="flex justify-between rounded-xl bg-darkSlate-800 px-4 py-2 box-border">
          <Input
            ref={inputRef}
            placeholder={t("instance.import_path") as string}
            icon={<div class="i-ri:archive-drawer" />}
            class="w-full rounded-full"
            onInput={(e) => setScanPath(e.target.value)}
            // disabled={}
          />
          <Button
            disabled={isAllImported()}
            onClick={() => {
              const dialogProperties = dialogPropertiesFromEntity(
                selectedEntity()
              );
              console.log(dialogProperties);
              window
                .openFileDialogExtended(dialogProperties)
                .then((files) => {});
            }}
          >
            <div class="i-ri:archive-drawer-fill" />
            <Trans key="general.browse_file" />
          </Button>
        </div>
        <div class="w-full bg-darkSlate-800 rounded-xl box-border flex flex-col overflow-hidden h-50">
          <div class="flex justify-between w-full bg-darkSlate-900 px-4 py-2 box-border">
            <Checkbox
              disabled={
                isAllImported() ||
                importedInstances().length > 0 ||
                Object.values(loadingInstances).filter((loading) => loading)
                  .length > 0
              }
              checked={areAllSelected()}
              onChange={(checked) => {
                selectAll(checked);
              }}
            />
            <span
              class="cursor-pointer"
              classList={{
                "text-darkSlate-600":
                  isAllImported() ||
                  importedInstances().length > 0 ||
                  Object.values(loadingInstances).filter((loading) => loading)
                    .length > 0,
              }}
              onClick={() => {
                if (
                  (!isAllImported() || importedInstances().length === 0) &&
                  Object.values(loadingInstances).filter((loading) => loading)
                    .length === 0
                )
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
            <Match when={(instances()?.data?.length || 0) > 0}>
              <div class="p-4 h-full w-full box-border flex flex-col gap-4">
                <For each={instances()?.data}>
                  {(instance, i) => (
                    <div class="flex justify-between">
                      <div class="flex gap-2 w-full">
                        <Checkbox
                          disabled={
                            importedInstances().includes(i()) ||
                            !!loadingInstances[i()]
                          }
                          checked={selectedInstancesIndexes[i()]}
                          onChange={(checked) => {
                            // eslint-disable-next-line solid/reactivity
                            setSelectedInstancesIndexes((prev) => ({
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
                          loadingInstances[i()] !== null &&
                          loadingInstances[i()] !== undefined
                        }
                      >
                        <div class="flex justify-between w-full">
                          <Show
                            when={isProgressKnown(
                              (loadingInstances[i()] as FETask).progress
                            )}
                          >
                            <div class="w-1/2 relative rounded-lg overflow-hidden bg-darkSlate-600">
                              <div
                                class="bg-green-500 text-xs absolute left-0 top-0 bottom-0"
                                style={{
                                  width: `${
                                    (
                                      (loadingInstances[i()] as FETask)
                                        .progress as {
                                        Known: number;
                                      }
                                    ).Known * 100
                                  }%`,
                                }}
                              />
                            </div>
                          </Show>
                          <Spinner />
                        </div>
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
        disabled={isAllImported()}
        onClick={() => {
          setIsLoading(true);
          props?.setIsLoading?.(true);
          const firstSelectedEntry = selectedEntires()[0];
          const firstSelectedEntryIndex = firstSelectedEntry[0];
          const parsedIndex = parseInt(firstSelectedEntryIndex, 10);

          importInstanceMutation.mutate({
            entity: selectedEntity(),
            index: parsedIndex,
            name: "", // TODO
          });
        }}
      >
        <Switch>
          <Match
            when={
              Object.values(selectedInstancesIndexes).filter((id) => id)
                .length > 0 && !isLoading()
            }
          >
            <div class="i-ri:folder-open-fill" />
            <Trans
              key="instance.import_instance_amount"
              options={{
                instances_amount: Object.values(
                  selectedInstancesIndexes
                ).filter((id) => id).length,
              }}
            />
          </Match>
          <Match
            when={
              Object.values(selectedInstancesIndexes).filter((id) => id)
                .length === 0 && !isLoading()
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
