import { rspc, queryClient } from "@/utils/rspcClient";
import {
  ImportEntityStatus,
  ImportableInstance,
  InvalidImportEntry
} from "@gd/core_module/bindings";
import { Button, Checkbox, Input, Tooltip } from "@gd/ui";
import {
  For,
  Match,
  Setter,
  Show,
  Switch,
  createEffect,
  createSignal
} from "solid-js";
import { createStore } from "solid-js/store";
import SingleCheckBox from "./SingleCheckBox";
import BeginImportStep from "./BeginImportStep";
import { Trans, useTransContext } from "@gd/i18n";
import { setTaskIds } from "@/utils/import";

const [step, setStep] = createSignal("selectionStep");
const [instances, setInstances] = createSignal([]);

const [globalInstances, setGlobalInstances] = createSignal<any[]>([]);
export { step, setStep, instances, setInstances, globalInstances };

const SingleEntity = (props: {
  entity: ImportEntityStatus;
  setEntity: Setter<ImportEntityStatus | undefined>;
}) => {
  const [t] = useTransContext();
  const [path, setPath] = createSignal<string | undefined>(undefined);
  const [inputValue, setInputValue] = createSignal(path());
  const [instance, setInstance] = createStore<{
    noResult: string | undefined;
    singleResult: ImportableInstance | undefined;
    multiResult: (ImportableInstance | InvalidImportEntry)[] | undefined;
  }>({
    noResult: undefined,
    singleResult: undefined,
    multiResult: undefined
  });

  const entityDefaultPath = rspc.createQuery(() => [
    "instance.getImportEntityDefaultPath",
    props.entity.entity
  ]);
  const scanImportableInstancesMutation = rspc.createMutation(
    ["instance.setImportScanTarget"],
    {
      onSettled: () => {
        queryClient.invalidateQueries({
          queryKey: ["instance.getImportScanStatus"]
        });
      }
    }
  );
  const importScanStatus = rspc.createQuery(() => [
    "instance.getImportScanStatus"
  ]);
  createEffect(() => {
    if (!entityDefaultPath.data) {
      setPath("");
    }

    setPath(entityDefaultPath.data!);
  });

  createEffect(() => {
    if (path()) {
      scanImportableInstancesMutation.mutate([
        props.entity.entity,
        path() as string
      ]);
    } else {
      scanImportableInstancesMutation.mutate([props.entity.entity, ""]);
    }
  });
  createEffect(() => {
    const status = importScanStatus.data;
    if (status) {
      const data = status.status;

      if (typeof data === "object") {
        if ("SingleResult" in data) {
          if ("Valid" in data.SingleResult) {
            const res = data.SingleResult;

            setGlobalInstances([res.Valid]);
            setInstance({
              singleResult: res.Valid,
              multiResult: undefined,
              noResult: undefined
            });
          }
        } else if ("MultiResult" in data) {
          const res = data.MultiResult;
          setGlobalInstances(
            res.map((e) => {
              if ("Valid" in e) {
                return e.Valid;
              } else {
                return e.Invalid;
              }
            })
          );
          setInstance({
            multiResult: res.map((e) => {
              if ("Valid" in e) {
                return e.Valid;
              } else {
                return e.Invalid;
              }
            }),
            singleResult: undefined,
            noResult: undefined
          });
        }
      } else {
        setInstance({
          noResult: data,
          singleResult: undefined,
          multiResult: undefined
        });
      }
    }
  });
  createEffect(() => {
    console.log(instances());
  });
  return (
    <>
      <div class="flex-1 w-full flex flex-col items-center justify-center p-4">
        <div class="flex flex-col items-start justify-start w-full gap-2">
          <span class="font-bold">
            {props.entity.entity} <Trans key="instance.import_path" />:
          </span>
          <div class="flex items-center w-full gap-2">
            <Input
              value={path()}
              onInput={(e) => {
                setInputValue(e.currentTarget.value);
              }}
              onBlur={() => {
                if (inputValue() && inputValue() !== path()) {
                  setPath(inputValue());
                }
              }}
              class="flex-1"
              inputColor="bg-darkSlate-800"
              icon={
                <div
                  onClick={() => {
                    setPath("");
                  }}
                  class="i-ri:close-line bg-darkSlate-50 hover:bg-white"
                />
              }
            />
            <div class="flex gap-2">
              <Show when={entityDefaultPath.data}>
                <Tooltip content={<Trans key="tooltip.reset" />}>
                  <div class="flex items-center justify-center p-2 bg-darkSlate-800 rounded-lg text-darkSlate-50 hover:text-white">
                    <div
                      onClick={async () => {
                        setPath(entityDefaultPath.data!);
                      }}
                      class="text-xl i-ri:arrow-go-back-fill"
                    />
                  </div>
                </Tooltip>
              </Show>
              <Show when={props.entity.selection_type === "directory"}>
                <Tooltip content={<Trans key="instance.select_path" />}>
                  <div class="flex items-center justify-center p-2 bg-darkSlate-800 rounded-lg text-darkSlate-50 hover:text-white">
                    <div
                      onClick={async () => {
                        const result = await window.openFileDialog({
                          title: t("instance.select_path"),
                          defaultPath: path() || "",
                          properties: ["openFile", "openDirectory"]
                        });

                        if (result.canceled) {
                          return;
                        }

                        setPath(result.filePaths[0]);
                      }}
                      class="text-xl i-ri:folder-line"
                    />
                  </div>
                </Tooltip>
              </Show>
              <Show when={props.entity.selection_type === "file"}>
                <Tooltip content={<Trans key="instance.select_zip" />}>
                  <div class="flex items-center justify-center p-2 bg-darkSlate-800 rounded-lg text-darkSlate-50 hover:text-white">
                    <div
                      onClick={async () => {
                        const result = await window.openFileDialog({
                          title: t("instance.select_zip"),
                          defaultPath: path() || "",
                          properties: ["openFile"],
                          filters: [
                            { name: "ZIP Files", extensions: ["zip"] },
                            { name: "All Files", extensions: ["*"] }
                          ]
                        });

                        if (result.canceled) {
                          return;
                        }

                        setPath(result.filePaths[0]);
                      }}
                      class="text-xl i-ri:file-zip-line"
                    />
                  </div>
                </Tooltip>
              </Show>
            </div>
          </div>
        </div>
        <div class="flex-1 w-full flex items-start justify-start mt-2 py-2 rounded-md bg-[#1D2028]">
          <Switch>
            <Match when={step() === "selectionStep"}>
              <Switch
                fallback={
                  <div class="w-full h-full flex items-center justify-center">
                    <p class="text-xl text-gray-500">
                      {path()
                        ? t("instance.no_instance_found")
                        : t("instance.select_path")}
                    </p>
                  </div>
                }
              >
                <Match when={typeof instance.multiResult !== "undefined"}>
                  <div class="h-full p-2 w-full flex flex-col gap-4">
                    <Checkbox
                      children={
                        <span class="text-sm text-[#8A8B8F]">
                          {t("instance.select_all_mods")}
                        </span>
                      }
                      checked={
                        instances().length === instance.multiResult?.length
                      }
                      indeterminate={instances().length !== 0}
                      onChange={(e) => {
                        if (e) {
                          setInstances(
                            typeof instance.multiResult !== "undefined"
                              ? (instance.multiResult.map(
                                  (e: any) => e.instance_name
                                ) as never[])
                              : []
                          );
                        } else {
                          setInstances([]);
                        }
                      }}
                    />
                    <div class="w-full h-[240px] overflow-y-auto flex flex-col gap-2">
                      <For each={instance.multiResult}>
                        {(entry) => (
                          <SingleCheckBox
                            title={(() => {
                              if ("instance_name" in entry) {
                                return entry.instance_name;
                              }
                            })()}
                            setList={setInstances}
                          />
                        )}
                      </For>
                    </div>
                  </div>
                </Match>
                <Match when={typeof instance.singleResult !== "undefined"}>
                  <For each={[instance.singleResult]}>
                    {() => (
                      <SingleCheckBox
                        title={instance.singleResult?.instance_name}
                        setList={setInstances}
                      />
                    )}
                  </For>
                </Match>
              </Switch>
            </Match>
            <Match when={step() === "importStep"}>
              <BeginImportStep instances={instances()} />
            </Match>
          </Switch>
        </div>
      </div>
      <div class="w-full flex justify-between items-center pt-6">
        <Button
          type="secondary"
          onClick={() => {
            props.setEntity(undefined);
            setStep("selectionStep");
            setInstances([]);
          }}
        >
          <Trans key="onboarding.go_back" />
        </Button>

        <Show when={step() === "selectionStep"} fallback={<div />}>
          <Button
            disabled={instances().length === 0}
            type="primary"
            onClick={() => {
              setTaskIds([]);
              setStep("importStep");
            }}
          >
            <Trans key="onboarding.begin_import" />
          </Button>
        </Show>
      </div>
    </>
  );
};
export default SingleEntity;

// GET_IMPORT_ENTITY_DEFAULT_PATH => returns an Option<String> of the default search path for the given import type
// SET_IMPORT_SCAN_TARGET => begins scanning at the given (path, import type). if GET_IMPORT_ENTITY_DEFAULT_PATH returns some you can call this immediately
// GET_IMPORT_SCAN_STATUS => gets the status of the current scan. the status includes scanning (if the scanner is currently active) and status, the current status of the scanner. the status can be NoResults, SingleResult(result), or MultiResult(Vec<result>). result is an ImportEntity which may either be valid and contain the filename and instance name, or invalid and contain the filename and a translation for the invalid reason. GET_IMPORT_SCAN_STATUS will fail if there is not currently an active scan.
// CANCEL_IMPORT_SCAN => stops any running scans, call this if the modal closes
// IMPORT_INSTANCE => starts importing an instance, returns the taskid of the prepare task
