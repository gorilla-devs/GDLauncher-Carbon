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
import { Trans, useTransContext } from "@gd/i18n"
import { ENTITIES } from "@/utils/constants"
import { useModal } from "../.."

const [instances, setInstances] = createSignal<string[]>([])
const [globalInstances, setGlobalInstances] = createSignal<any[]>([])
export { instances, setInstances, globalInstances }

const SingleEntity = (props: {
  entity: ImportEntityStatus
  setEntity: Setter<ImportEntityStatus | undefined>
}) => {
  const [t] = useTransContext()
  const modalsContext = useModal()
  const [path, setPath] = createSignal<string | undefined>(undefined)
  const [inputValue, setInputValue] = createSignal(path())
  const [isImporting, setIsImporting] = createSignal(false)

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

  const importInstanceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.importInstance"]
  }))

  const handleBeginImport = async () => {
    setIsImporting(true)

    // Fire all import requests in parallel (backend will queue them)
    const importPromises = instances().map((filename) => {
      const index = globalInstances().findIndex(
        (x: any) => x.filename === filename
      )
      return importInstanceMutation
        .mutateAsync({
          name: filename,
          index
        })
        .catch((error) => {
          console.error("Import failed:", error)
        })
    })

    // Wait for all mutation calls to be sent
    await Promise.all(importPromises)

    // Close modal after all requests are fired
    modalsContext?.closeModal()
  }

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
      {/* Fixed Header - section title and path input */}
      <div class="flex w-full shrink-0 flex-col gap-3 p-4">
        {/* Section Header */}
        <div class="flex w-full items-center">
          <div class="border-t-1 border-lightSlate-400 flex-1 border-solid" />
          <span class="text-lightSlate-400 flex items-center gap-2 px-3 text-base">
            <img
              src={ENTITIES[props.entity.entity].icon}
              alt={props.entity.entity}
              class="h-4 w-4"
            />
            <Trans key={ENTITIES[props.entity.entity].translation} />
          </span>
          <div class="border-t-1 border-lightSlate-400 flex-1 border-solid" />
        </div>

        {/* Path Label */}
        <label class="text-lightSlate-400 text-xs font-medium">
          <Trans key="instances:_trn_import_path" />
        </label>
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
                              name: "Modpack Files",
                              extensions:
                                props.entity.entity === "CurseForgeZip"
                                  ? ["zip"]
                                  : props.entity.entity === "GDLPack"
                                    ? ["gdlpack"]
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

      {/* Selection Header - fixed outside scroll */}
      <Show when={typeof instance.multiResult !== "undefined"}>
        <div class="flex shrink-0 items-center justify-between px-4 pb-3">
          <Checkbox
            checked={instances().length === instance.multiResult?.length}
            indeterminate={
              instances().length > 0 &&
              instances().length < (instance.multiResult?.length || 0)
            }
            onChange={(e) => {
              if (e) {
                setInstances(
                  typeof instance.multiResult !== "undefined"
                    ? (instance.multiResult.map(
                        (entry: any) => entry.filename
                      ) as string[])
                    : []
                )
              } else {
                setInstances([])
              }
            }}
          >
            <span class="text-lightSlate-400 text-sm">
              {instances().length} / {instance.multiResult?.length}{" "}
              <Trans key="instances:_trn_import_select_all_instances" />
            </span>
          </Checkbox>
        </div>
      </Show>

      {/* Scrollable Content - results area */}
      <div class="flex-1 overflow-y-auto px-4 pb-4 pt-1">
        <Switch
          fallback={
            <>
              <Show when={importScanStatus.data?.scanning}>
                <div class="flex h-40 w-full items-center justify-center">
                  <Spinner />
                </div>
              </Show>
              <Show
                when={
                  importScanStatus.data?.status === "NoResults" &&
                  !importScanStatus.data?.scanning
                }
              >
                <div class="bg-darkSlate-800 flex h-40 w-full flex-col items-center justify-center gap-2 rounded-lg">
                  <div class="i-hugeicons:folder-search text-lightSlate-600 text-3xl" />
                  <p class="text-lightSlate-500 text-sm">
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
            {/* Instance List */}
            <div class="flex flex-col gap-2">
              <For each={instance.multiResult}>
                {(entry) => (
                  <SingleCheckBox
                    title={
                      "instance_name" in entry ? entry.instance_name : undefined
                    }
                    filename={"filename" in entry ? entry.filename : undefined}
                    setList={setInstances}
                  />
                )}
              </For>
            </div>
          </Match>
          <Match when={typeof instance.singleResult !== "undefined"}>
            <div class="flex flex-col gap-2">
              <SingleCheckBox
                title={instance.singleResult?.instance_name}
                filename={instance.singleResult?.filename}
                setList={setInstances}
              />
            </div>
          </Match>
          <Match when={instance.isLoading === true}>
            <div class="flex h-40 w-full items-center justify-center">
              <Spinner />
            </div>
          </Match>
        </Switch>
      </div>

      {/* Fixed Footer - buttons */}
      <div class="flex w-full shrink-0 items-center justify-between px-4 pb-4">
        <Button
          type="secondary"
          onClick={() => {
            props.setEntity(undefined)
            setInstances([])
          }}
        >
          <Trans key="onboarding:_trn_go_back" />
        </Button>

        <Button
          disabled={instances().length === 0 || isImporting()}
          loading={isImporting()}
          type="primary"
          onClick={handleBeginImport}
        >
          <Trans key="onboarding:_trn_begin_import" />
        </Button>
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
