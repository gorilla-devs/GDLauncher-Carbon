import { rspc, queryClient } from "@/utils/rspcClient"
import { extractErrorDisplay } from "@/utils/rspcErrorText"
import {
  ImportEntityStatus,
  ImportableInstance
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
  createSignal,
  onCleanup
} from "solid-js"
import { createAsyncEffect } from "@/utils/asyncEffect"
import { createStore } from "solid-js/store"
import SingleCheckBox from "./SingleCheckBox"
import { createImportBatch } from "./importBatch"
import { scanRows, type ScanRow } from "./importSelection"
import { Trans, useTransContext } from "@gd/i18n"
import { getImportErrorTranslationKey } from "@gd/i18n/helpers"
import { ENTITIES } from "@/utils/constants"
import { useModal } from "../.."

const [instances, setInstances] = createSignal<string[]>([])
const [globalInstances, setGlobalInstances] = createSignal<ScanRow[]>([])
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
  const [session, setSession] = createSignal<string>()
  const [requestError, setRequestError] = createSignal<string>()
  const cancelScan = rspc.createMutation(() => ({
    mutationKey: ["instance.cancelImportScan"]
  }))
  onCleanup(() => {
    ++scanRequest
    setSession(undefined)
    void cancelScan.mutateAsync(undefined)
  })

  const [instance, setInstance] = createStore<{
    noResult: string | undefined
    singleResult: (ImportableInstance & ScanRow) | undefined
    multiResult: ScanRow[] | undefined
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

  const runBatch = createImportBatch()
  const handleBeginImport = async () => {
    const currentSession = session()
    if (!currentSession) return
    setRequestError(undefined)
    await runBatch({
      session: currentSession,
      selectionIds: instances(),
      rows: globalInstances(),
      submit: (request) => importInstanceMutation.mutateAsync(request),
      currentSession: session,
      setBusy: setIsImporting,
      removeSelections: (ids) =>
        setInstances((selected) => selected.filter((id) => !ids.includes(id))),
      // The translated text already says to rescan; the causes (English,
      // and for a refused request also in the global toast) go to the log.
      onUncertain: (errors) => {
        console.warn("[import] Batch outcome uncertain:", errors)
        setRequestError(t("instances:_trn_import_batch_uncertain"))
      },
      onSuccess: () => modalsContext?.closeModal()
    })
  }

  let scanRequest = 0
  const startScan = async (currentPath: string | undefined) => {
    const request = ++scanRequest
    setSession(undefined)
    setInstances([])
    setGlobalInstances([])
    setRequestError(undefined)
    if (!currentPath) {
      await cancelScan.mutateAsync(undefined)
      return undefined
    }
    const newSession = await scanImportableInstancesMutation.mutateAsync([
      props.entity.entity,
      currentPath
    ])
    if (request === scanRequest) setSession(newSession)
  }

  const retryScan = async () => {
    try {
      await startScan(path())
    } catch (error) {
      setRequestError(extractErrorDisplay(error))
    }
  }

  const importScanStatus = rspc.createQuery(() => ({
    queryKey: ["instance.getImportScanStatus"]
  }))

  const rowWarnings = (entry: ImportableInstance | undefined) =>
    entry?.warnings.map((warning) =>
      t(getImportErrorTranslationKey(warning.translation))
    )

  const selectableFilenames = () =>
    (instance.multiResult ?? [])
      .filter((entry) => "filename" in entry)
      .map((entry) => entry.selectionId)

  createEffect(() => {
    if (!entityDefaultPath.data) {
      setPath("")
    }

    setPath(entityDefaultPath.data!)
  })

  createAsyncEffect<string>((isStale, _prevPath) => {
    const currentPath = path()

    const mutation = startScan(currentPath)

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
        if (!isStale()) setRequestError(extractErrorDisplay(error))
      })

    return currentPath
  }, undefined)

  createEffect(() => {
    const status = importScanStatus.data
    if (!status || !session() || status.session !== session()) {
      setInstance({
        noResult: undefined,
        singleResult: undefined,
        multiResult: undefined,
        isLoading: !!path()
      })
      return
    }
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
          const res = data.SingleResult
          if ("Valid" in res) {
            setGlobalInstances(scanRows([res], status.session))
            setInstance({
              singleResult: scanRows(
                [res],
                status.session
              )[0] as ImportableInstance & ScanRow,
              multiResult: undefined,
              noResult: undefined,
              isLoading: false
            })
          } else {
            setGlobalInstances(scanRows([res], status.session))
            setInstance({
              singleResult: undefined,
              multiResult: scanRows([res], status.session),
              noResult: undefined,
              isLoading: false
            })
          }
        } else if ("MultiResult" in data) {
          const entries = scanRows(data.MultiResult, status.session)
          setGlobalInstances(entries)
          setInstance({
            multiResult: entries,
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
                                props.entity.entity === "CurseForgeZip" ||
                                props.entity.entity === "MultiMCZip"
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

      <Show
        when={
          requestError() ||
          (importScanStatus.data?.session === session() &&
            importScanStatus.data?.error)
        }
      >
        <div role="alert" class="px-4 pb-3">
          <p>{t("instances:_trn_import_scan_failed")}</p>
          <p class="break-words text-sm">
            {requestError() || importScanStatus.data?.error}
          </p>
          <Button type="secondary" onClick={retryScan}>
            {t("instances:_trn_import_retry")}
          </Button>
        </div>
      </Show>
      {/* Selection Header - fixed outside scroll */}
      <Show
        when={
          typeof instance.multiResult !== "undefined" &&
          selectableFilenames().length > 0
        }
      >
        <div class="flex shrink-0 items-center justify-between px-4 pb-3">
          <Checkbox
            checked={instances().length === selectableFilenames().length}
            indeterminate={
              instances().length > 0 &&
              instances().length < selectableFilenames().length
            }
            onChange={(e) => {
              setInstances(e ? selectableFilenames() : [])
            }}
          >
            <span class="text-lightSlate-400 text-sm">
              {instances().length} / {selectableFilenames().length}{" "}
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
                  !importScanStatus.data?.scanning &&
                  !importScanStatus.data?.error
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
                {(entry) =>
                  "filename" in entry ? (
                    <SingleCheckBox
                      title={entry.instance_name}
                      filename={entry.filename}
                      selectionId={entry.selectionId}
                      warnings={rowWarnings(entry)}
                      setList={setInstances}
                    />
                  ) : (
                    <SingleCheckBox
                      title={entry.name}
                      invalidReason={t(
                        getImportErrorTranslationKey(entry.reason.translation)
                      )}
                    />
                  )
                }
              </For>
            </div>
          </Match>
          <Match when={typeof instance.singleResult !== "undefined"}>
            <div class="flex flex-col gap-2">
              <SingleCheckBox
                title={instance.singleResult?.instance_name}
                filename={instance.singleResult?.filename}
                selectionId={instance.singleResult?.selectionId}
                warnings={rowWarnings(instance.singleResult)}
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
          disabled={
            instances().length === 0 ||
            isImporting() ||
            !session() ||
            importScanStatus.data?.scanning ||
            importScanStatus.data?.session !== session()
          }
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
