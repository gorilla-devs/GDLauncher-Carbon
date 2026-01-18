import { useModal } from "../.."
import { Button, Input, Spinner } from "@gd/ui"
import { rspc } from "@/utils/rspcClient"
import { For, Match, Show, Switch, createEffect, createSignal } from "solid-js"
import { ImportEntity, ImportEntityStatus } from "@gd/core_module/bindings"
import EntityCard from "@/components/Card/EntityCard"
import SingleEntity, { setInstances } from "./SingleEntity"

import { Trans, useTransContext } from "@gd/i18n"
import { ENTITIES } from "@/utils/constants"
import { parseShareInput } from "@/utils/searchQueryParser"

// Helper to extract error code from rspc error
const getErrorCode = (error: unknown): string | null => {
  try {
    if (
      error &&
      typeof error === "object" &&
      "message" in error &&
      typeof error.message === "string"
    ) {
      const parsed = JSON.parse(error.message)
      if (parsed?.cause && Array.isArray(parsed.cause)) {
        for (const segment of parsed.cause) {
          if (segment?.code) {
            return segment.code
          }
        }
      }
    }
  } catch {
    // ignore parse errors
  }
  return null
}

// Map error codes to translation keys for share import errors
type ShareErrorKey =
  | "instances:_trn_share_errors.share_not_found"
  | "instances:_trn_share_errors.max_downloads_exceeded"
  | "instances:_trn_share_errors.network_error"
  | "instances:_trn_share_errors.unknown"

const getShareImportErrorKey = (code: string | null): ShareErrorKey => {
  switch (code) {
    case "SHARE_NOT_FOUND":
      return "instances:_trn_share_errors.share_not_found"
    case "MAX_DOWNLOADS_EXCEEDED":
      return "instances:_trn_share_errors.max_downloads_exceeded"
    case "NETWORK_ERROR":
      return "instances:_trn_share_errors.network_error"
    default:
      return "instances:_trn_share_errors.unknown"
  }
}

const LAUNCHER_ENTITIES: ImportEntity[] = [
  "LegacyGDLauncher",
  "ATLauncher",
  "CurseForge",
  "FTB",
  "MultiMC",
  "Technic",
  "PrismLauncher",
  "Modrinth"
]

const FILE_ENTITIES: ImportEntity[] = ["CurseForgeZip", "MRPack"]

interface Props {
  prevStep: () => void
  isImportInstance?: boolean
}

const [currentEntity, setCurrentEntity] = createSignal<
  ImportEntityStatus | undefined
>()

const ThirdStep = (props: Props) => {
  const modalsContext = useModal()
  const [t] = useTransContext()
  const rspcContext = rspc.useContext()

  const [entity, setEntity] = createSignal<ImportEntityStatus | undefined>()

  const entities = rspc.createQuery(() => ({
    queryKey: ["instance.getImportableEntities"]
  }))

  const [shareInput, setShareInput] = createSignal<string>("")
  const [parsedShareCode, setParsedShareCode] = createSignal<string | null>(null)
  const [isValidating, setIsValidating] = createSignal(false)
  const [isCodeValid, setIsCodeValid] = createSignal(false)
  const [validationError, setValidationError] = createSignal<string | null>(
    null
  )

  // Auto-parse and validate when input changes
  createEffect(() => {
    const input = shareInput()
    const parsed = parseShareInput(input)
    setParsedShareCode(parsed)

    if (parsed) {
      validateCode(parsed)
    } else {
      setIsCodeValid(false)
      setValidationError(null)
    }
  })

  const validateCode = async (code: string) => {
    setIsValidating(true)
    setValidationError(null)
    try {
      const isValid = await rspcContext.client.query([
        "instance.validateShareCode",
        code
      ])
      setIsCodeValid(isValid)
      if (!isValid) {
        setValidationError(t("instances:_trn_share_errors.share_not_found"))
      }
    } catch (err) {
      setIsCodeValid(false)
      const errorCode = getErrorCode(err)
      setValidationError(t(getShareImportErrorKey(errorCode)))
    } finally {
      setIsValidating(false)
    }
  }

  const handlePreview = () => {
    const code = parsedShareCode()
    if (!code) return
    modalsContext?.openModal({ name: "sharePreview" }, { shareCode: code })
  }

  const handleClickEntity = (ent: ImportEntityStatus) => {
    if (ent.supported) {
      // Reset instances when switching to a different entity
      if (currentEntity() && currentEntity()?.entity !== ent.entity) {
        setInstances([])
      }
      setEntity(ent)
      setCurrentEntity(ent)
    }
  }

  return (
    <div
      class={`flex flex-col ${
        props.isImportInstance
          ? "h-[600px] w-full"
          : "w-120 lg:w-160 h-full pt-6"
      } box-border`}
    >
      <Switch>
        <Match when={entities.isLoading}>
          <div class="flex h-full w-full items-center justify-center">
            <Spinner class="h-10 w-10" />
          </div>
        </Match>
        <Match when={entity()}>
          <SingleEntity entity={entity()!} setEntity={setEntity} />
        </Match>
        <Match when={!entity()}>
          <div
            class={`flex w-full flex-1 flex-col overflow-y-auto ${
              props.isImportInstance ? "px-4 pt-4" : ""
            }`}
          >
            <div class="flex w-full items-center">
              <div class="border-t-1 border-lightSlate-400 flex-1 border-solid" />
              <span class="text-lightSlate-400 flex items-center gap-2 px-3 text-base">
                <div class="i-hugeicons:share-08 text-primary-500 text-sm" />
                <Trans key="instances:_trn_import_share_code" />
              </span>
              <div class="border-t-1 border-lightSlate-400 flex-1 border-solid" />
            </div>
            <div class="mt-3 mb-8 flex flex-col">
              <div class="relative flex items-center gap-2">
                <Input
                  placeholder={t("instances:_trn_share_preview.input_placeholder") || "Share code or gdl.gg link"}
                  class={`flex-1 shrink-0 rounded-md ${isCodeValid() ? "ring-2 ring-green-500" : validationError() ? "ring-2 ring-red-500" : ""}`}
                  inputColor="bg-darkSlate-800"
                  value={shareInput()}
                  onInput={(e) => {
                    setShareInput(e.target.value)
                  }}
                />
                <div class="w-32">
                  <Button
                    fullWidth
                    disabled={!parsedShareCode() || !isCodeValid() || isValidating()}
                    loading={isValidating()}
                    onClick={handlePreview}
                  >
                    <div class="i-ri:eye-line" />
                    <Trans key="instances:_trn_share_preview.preview_button" />
                  </Button>
                </div>
                <Show when={validationError()}>
                  <span class="text-red-400 text-xs absolute -bottom-5 left-0">
                    {validationError()}
                  </span>
                </Show>
              </div>
            </div>
            <div class="flex w-full items-center">
              <div class="border-t-1 border-lightSlate-400 flex-1 border-solid" />
              <span class="text-lightSlate-400 flex items-center gap-2 px-3 text-base">
                <div class="i-hugeicons:rocket-02 text-primary-500 text-sm" />
                <Trans key="instances:_trn_import_from_launcher" />
              </span>
              <div class="border-t-1 border-lightSlate-400 flex-1 border-solid" />
            </div>
            <ul class="mt-3 mb-8 grid grid-cols-4 gap-1.5 p-0">
              <For
                each={entities.data
                  ?.filter((e) => LAUNCHER_ENTITIES.includes(e.entity))
                  .sort(
                    (a, b) =>
                      (b.supported === true ? 1 : 0) -
                      (a.supported === true ? 1 : 0)
                  )}
              >
                {(entity) => (
                  <EntityCard
                    entity={entity}
                    icon={ENTITIES[entity.entity].icon}
                    translation={ENTITIES[entity.entity].translation}
                    onClick={[handleClickEntity, entity]}
                  />
                )}
              </For>
            </ul>
            <div class="flex w-full items-center">
              <div class="border-t-1 border-lightSlate-400 flex-1 border-solid" />
              <span class="text-lightSlate-400 flex items-center gap-2 px-3 text-base">
                <div class="i-hugeicons:file-zip text-primary-500 text-sm" />
                <Trans key="instances:_trn_import_from_file" />
              </span>
              <div class="border-t-1 border-lightSlate-400 flex-1 border-solid" />
            </div>
            <ul class="mt-3 grid grid-cols-4 gap-1.5 p-0">
              <For
                each={entities.data
                  ?.filter((e) => FILE_ENTITIES.includes(e.entity))
                  .sort(
                    (a, b) =>
                      (b.supported === true ? 1 : 0) -
                      (a.supported === true ? 1 : 0)
                  )}
              >
                {(entity) => (
                  <EntityCard
                    entity={entity}
                    icon={ENTITIES[entity.entity].icon}
                    translation={ENTITIES[entity.entity].translation}
                    onClick={[handleClickEntity, entity]}
                  />
                )}
              </For>
            </ul>
          </div>
          <Show when={!props.isImportInstance}>
            <div class="flex w-full justify-between">
              <Button
                onClick={() => {
                  props.prevStep()
                }}
                size="large"
                type="secondary"
              >
                <Trans key="onboarding:_trn_prev" />
              </Button>
              <Button
                onClick={() => {
                  modalsContext?.closeModal()
                }}
                size="large"
                type="primary"
              >
                <Trans key="onboarding:_trn_skip" />
              </Button>
            </div>
          </Show>
        </Match>
      </Switch>
    </div>
  )
}

export default ThirdStep
