import { rspc, queryClient } from "@/utils/rspcClient"
import {
  ImportEntityStatus,
  ImportableInstance,
  InvalidImportEntry
} from "@gd/core_module/bindings"
import {
  Button,
  Checkbox,
  Input,
  Spinner,
  Tooltip,
  TooltipContent,
  TooltipTrigger
} from "@gd/ui"
import {
  For,
  Match,
  Setter,
  Show,
  Switch,
  createEffect,
  createSignal
} from "solid-js"
import { createAsyncEffect } from "@/utils/asyncEffect"
import { createStore } from "solid-js/store"
import SingleCheckBox from "./SingleCheckBox"
import BeginImportStep from "./BeginImportStep"
import { Trans, useTransContext } from "@gd/i18n"
import { setTaskIds } from "@/utils/import"

const [step, setStep] = createSignal("selectionStep")
const [instances, setInstances] = createSignal([])
const [globalInstances, setGlobalInstances] = createSignal<any[]>([])
export { step, setStep, instances, setInstances, globalInstances }

const SingleEntity = (props: {
  entity: ImportEntityStatus
  setEntity: Setter<ImportEntityStatus | undefined>
}) => {
  const [t] = useTransContext()
  const [path, setPath] = createSignal<string | undefined>(undefined)
  const [inputValue, setInputValue] = createSignal(path())

  const [instance, setInstance] = createStore<{
    noResult: string | undefined
    singleResult: ImportableInstance | undefined
    multiResult: (ImportableInstance | InvalidImportEntry)[] | undefined
    isLoading?: boolean
  }>({
    noResult: undefined,
    singleResult: undefined,
    multiResult: undefined,
    isLoading: false
  })

  const entityDefaultPath = rspc.createQuery(() => ({
    queryKey: ["instance.getImportEntityDefaultPath", props.entity.entity]
  }))
  const scanImportableInstancesMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.setImportScanTarget"]
  }))

  const importScanStatus = rspc.createQuery(() => ({
    queryKey: ["instance.getImportScanStatus"]
  }))

  createEffect(() => {
    if (!entityDefaultPath.data) {
      setPath("")
    }

    setPath(entityDefaultPath.data!)
  })

  createAsyncEffect<string>((isStale, _prevPath) => {
    const currentPath = path()

    const mutation = currentPath
      ? scanImportableInstancesMutation.mutateAsync([
          props.entity.entity,
          currentPath
        ])
      : scanImportableInstancesMutation.mutateAsync([props.entity.entity, ""])

    mutation
      .then(() => {
        // Check if path hasn't changed during async operation
        if (!isStale()) {
          queryClient.invalidateQueries({
            queryKey: ["instance.getImportScanStatus"]
          })
        }
      })
      .catch((error) => {
        console.error("Failed to scan importable instances:", error)
      })

    return currentPath
  }, undefined)

  createEffect(() => {
    const status = importScanStatus.data
    if (status) {
      const data = status.status
      if (status.scanning) {
        setInstance({
          isLoading: true,
          noResult: undefined,
          singleResult: undefined,
          multiResult: undefined
        })
      }
      if (typeof data === "object") {
        if ("SingleResult" in data) {
          if ("Valid" in data.SingleResult) {
            const res = data.SingleResult
            setGlobalInstances([res.Valid])
            setInstance({
              singleResult: res.Valid,
              multiResult: undefined,
              noResult: undefined,
              isLoading: false
            })
          }
        } else if ("MultiResult" in data) {
          const res = data.MultiResult
          setGlobalInstances(
            res.map((e) => {
              if ("Valid" in e) {
                return e.Valid
              } else {
                return e.Invalid
              }
            })
          )
          setInstance({
            multiResult: res.map((e) => {
              if ("Valid" in e) {
                return e.Valid
              } else {
                return e.Invalid
              }
            }),
            singleResult: undefined,
            noResult: undefined,
            isLoading: false
          })
        }
      } else {
        setInstance({
          noResult: data,
          singleResult: undefined,
          multiResult: undefined,
          isLoading: false
        })
      }
    }
  })

  return (
    <div class="flex h-full flex-col">
      {/* Fixed Header - path input */}
      <div class="flex w-full shrink-0 flex-col items-start justify-start gap-2 p-4">
        <span class="font-bold">
          {props.entity.entity} <Trans key="instances:_trn_import_path" />:
        </span>
        <div class="flex w-full items-center gap-2">
          <Input
            value={path()}
            onInput={(e) => {
              setInputValue(e.currentTarget.value)
            }}
            onBlur={() => {
              if (inputValue() && inputValue() !== path()) {
                setPath(inputValue())
              }
            }}
            class="flex-1"
            inputColor="bg-darkSlate-800"
            icon={
              <div
                class="i-hugeicons:cancel-01 bg-darkSlate-50 hover:bg-white"
                onClick={() => {
                  setPath("")
                }}
              />
            }
          />
          <div class="flex gap-2">
            <Show when={entityDefaultPath.data}>
              <Tooltip>
                <TooltipTrigger>
                  <div class="bg-darkSlate-800 text-lightSlate-700 hover:text-lightSlate-50 flex items-center justify-center rounded-lg p-2">
                    <div
                      class="i-hugeicons:arrow-turn-backward text-xl"
                      onClick={async () => {
                        setPath(entityDefaultPath.data!)
                      }}
                    />
                  </div>
                </TooltipTrigger>
                <TooltipContent>
                  <Trans key="ui:_trn_tooltip.reset" />
                </TooltipContent>
              </Tooltip>
            </Show>
            <Show when={props.entity.selection_type === "directory"}>
              <Tooltip>
                <TooltipTrigger>
                  <div class="bg-darkSlate-800 text-lightSlate-700 hover:text-lightSlate-50 flex items-center justify-center rounded-lg p-2">
                    <div
                      class="i-hugeicons:folder-02"
                      onClick={async () => {
                        const result = await window.openFileDialog({
                          title: t("instances:_trn_select_path"),
                          defaultPath: path() || "",
                          properties: ["openFile", "openDirectory"]
                        })

                        if (result.canceled) {
                          return
                        }

                        setPath(result.filePaths[0])
                      }}
                    />
                  </div>
                </TooltipTrigger>
                <TooltipContent>
                  <Trans key="instances:_trn_select_path" />
                </TooltipContent>
              </Tooltip>
            </Show>
            <Show when={props.entity.selection_type === "file"}>
              <Tooltip>
                <TooltipTrigger>
                  <div class="bg-darkSlate-800 text-lightSlate-700 hover:text-lightSlate-50 flex items-center justify-center rounded-lg p-2">
                    <div
                      class="i-hugeicons:zip-02"
                      onClick={async () => {
                        const result = await window.openFileDialog({
                          title: t("instances:_trn_select_zip"),
                          defaultPath: path() || "",
                          properties: ["openFile"],
                          filters: [
                            {
                              name: "ZIP Files",
                              extensions:
                                props.entity.entity === "CurseForgeZip"
                                  ? ["zip"]
                                  : ["mrpack"]
                            },
                            { name: "All Files", extensions: ["*"] }
                          ]
                        })

                        if (result.canceled) {
                          return
                        }

                        setPath(result.filePaths[0])
                      }}
                    />
                  </div>
                </TooltipTrigger>
                <TooltipContent>
                  <Trans key="instances:_trn_select_zip" />
                </TooltipContent>
              </Tooltip>
            </Show>
          </div>
        </div>
      </div>

      {/* Scrollable Content - results area */}
      <div class="mx-4 mb-4 flex-1 overflow-y-auto">
        <div class="flex w-full flex-col items-start justify-start rounded-md bg-[#1D2028] py-2">
          <Switch>
            <Match when={step() === "selectionStep"}>
              <Switch
                fallback={
                  <>
                    <Show when={importScanStatus.data?.scanning}>
                      <div class="flex h-full w-full items-center justify-center">
                        <Spinner />
                      </div>
                    </Show>
                    <Show
                      when={
                        importScanStatus.data?.status === "NoResults" &&
                        !importScanStatus.data?.scanning
                      }
                    >
                      <div class="flex h-full w-full items-center justify-center">
                        <p class="text-xl text-gray-500">
                          {path()
                            ? t("instances:_trn_no_instance_found")
                            : t("instances:_trn_select_path")}
                        </p>
                      </div>
                    </Show>
                  </>
                }
              >
                <Match when={typeof instance.multiResult !== "undefined"}>
                  <div class="flex h-full w-full flex-col gap-4 p-2">
                    <Checkbox
                      children={
                        <span class="text-sm text-[#8A8B8F]">
                          {t("content:_trn_select_all_mods")}
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
                          )
                        } else {
                          setInstances([])
                        }
                      }}
                    />
                    <div class="flex h-[240px] w-full flex-col gap-2 overflow-y-auto">
                      <For each={instance.multiResult}>
                        {(entry) => (
                          <SingleCheckBox
                            title={(() => {
                              if ("instance_name" in entry) {
                                return entry.instance_name
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
                <Match when={instance.isLoading === true}>
                  <div class="flex h-full w-full items-center justify-center">
                    <div class="i-formkit:spinner h-10 w-10 animate-spin text-sky-800" />
                  </div>
                </Match>
              </Switch>
            </Match>
            <Match when={step() === "importStep"}>
              <BeginImportStep instances={instances()} />
            </Match>
          </Switch>
        </div>
      </div>

      {/* Fixed Footer - buttons */}
      <div class="flex w-full shrink-0 items-center justify-between px-4 pb-4">
        <Button
          type="secondary"
          onClick={() => {
            props.setEntity(undefined)
            setStep("selectionStep")
            setInstances([])
          }}
        >
          <Trans key="onboarding:_trn_go_back" />
        </Button>

        <Show when={step() === "selectionStep"} fallback={<div />}>
          <Button
            disabled={instances().length === 0}
            type="primary"
            onClick={() => {
              setTaskIds([])
              setStep("importStep")
            }}
          >
            <Trans key="onboarding:_trn_begin_import" />
          </Button>
        </Show>
      </div>
    </div>
  )
}
export default SingleEntity

// GET_IMPORT_ENTITY_DEFAULT_PATH => returns an Option<String> of the default search path for the given import type
// SET_IMPORT_SCAN_TARGET => begins scanning at the given (path, import type). if GET_IMPORT_ENTITY_DEFAULT_PATH returns some you can call this immediately
// GET_IMPORT_SCAN_STATUS => gets the status of the current scan. the status includes scanning (if the scanner is currently active) and status, the current status of the scanner. the status can be NoResults, SingleResult(result), or MultiResult(Vec<result>). result is an ImportEntity which may either be valid and contain the filename and instance name, or invalid and contain the filename and a translation for the invalid reason. GET_IMPORT_SCAN_STATUS will fail if there is not currently an active scan.
// CANCEL_IMPORT_SCAN => stops any running scans, call this if the modal closes
// IMPORT_INSTANCE => starts importing an instance, returns the taskid of the prepare task
