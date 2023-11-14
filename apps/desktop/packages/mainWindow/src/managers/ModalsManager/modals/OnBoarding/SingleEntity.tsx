import { rspc, queryClient } from "@/utils/rspcClient";
import {
  ImportEntityStatus,
  ImportableInstance,
  InvalidImportEntry
} from "@gd/core_module/bindings";
import { Button, Checkbox, Input } from "@gd/ui";
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

const [step, setStep] = createSignal("selectionStep");
const [instances, setInstances] = createSignal([]);
export { step, setStep, instances, setInstances };

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
            setInstance({
              singleResult: res.Valid,
              multiResult: undefined,
              noResult: undefined
            });
          }
        } else if ("MultiResult" in data) {
          const res = data.MultiResult;
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

  return (
    <>
      <div class="flex-1 w-full flex flex-col items-center justify-center p-4">
        <div class="flex items-center justify-between w-full gap-2">
          <span class="font-bold">
            <Trans
              options={{ defaultValue: "Scan target path:" }}
              key="onboarding.scan_target_path"
            />
          </span>
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
            class="flex-1 border-2 border-solid border-zinc-500"
            icon={
              <div class="flex gap-2">
                <div
                  onClick={async () => {
                    const result = await window.openFileDialog({
                      title: "Select Runtime Path",
                      defaultPath: path() || "",
                      properties: ["openFile", "openDirectory"]
                    });

                    if (result.canceled) {
                      return;
                    }

                    setPath(result.filePaths[0]);
                  }}
                  class="i-ic:round-folder text-2xl text-yellow-300 cursor-pointer"
                />
                <div
                  onClick={async () => {
                    const result = await window.openFileDialog({
                      title: "Select Runtime Path",
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
                  class="i-solar:file-bold text-2xl text-blue-500 cursor-pointer"
                />
              </div>
            }
          />
        </div>

        <div class="flex-1 w-full flex items-start justify-start border-2 border-gray-500 mt-2 py-2 rounded-md border-solid">
          <Switch>
            <Match when={step() === "selectionStep"}>
              <Switch
                fallback={
                  <div class="w-full h-full flex items-center justify-center">
                    <p class="text-xl text-gray-500">
                      {path()
                        ? "No Instances found on this path"
                        : "select a path"}
                    </p>
                  </div>
                }
              >
                <Match when={typeof instance.multiResult !== "undefined"}>
                  <div class="h-full p-2 w-full flex flex-col gap-4">
                    <Checkbox
                      children={
                        <span class="text-sm">
                          {t("instance.select_all_mods")}
                        </span>
                      }
                      checked={instances().length !== 0}
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
                    <div class="w-full h-[90%] overflow-hidden flex flex-col gap-2">
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
          <Trans
            options={{ defaultValue: "Go back" }}
            key="onboarding.go_back"
          />
        </Button>

        <span class="font-bold">{props.entity.entity}</span>
        <Show when={step() === "selectionStep"} fallback={<div />}>
          <Button
            disabled={instances().length === 0}
            type="primary"
            onClick={() => {
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
