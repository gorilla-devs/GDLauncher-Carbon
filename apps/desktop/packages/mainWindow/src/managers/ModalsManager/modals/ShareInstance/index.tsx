import {
  Button,
  toast,
  Input,
  Progress,
  Select,
  SelectTrigger,
  SelectContent,
  SelectItem,
  SelectValue,
  CopyText,
  Collapsable,
  Checkbox
} from "@gd/ui"
import { ModalProps, useModal } from "../.."
import ModalLayout from "../../ModalLayout"
import { apiUrl, rspc } from "@/utils/rspcClient"
import {
  createEffect,
  createMemo,
  createSignal,
  For,
  Match,
  onCleanup,
  Show,
  Switch
} from "solid-js"
import {
  FEShareEntry,
  FEShareEntryKind,
  FEWaitForInstanceShareResponse
} from "@gd/core_module/bindings"
import { Trans, useTransContext, type NamespacedTranslationKey } from "@gd/i18n"
import { useNavigate } from "@solidjs/router"
import {
  MAX_DOWNLOADS_LIMIT,
  parseMaxDownloads,
  validateMaxDownloads
} from "@/utils/validation"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import VerificationRequiredPlaceholder from "@/components/VerificationRequiredPlaceholder"
import { getErrorCode } from "@/components/SharePreviewContent"
import { formatBytes } from "@/utils/formatBytes"

const PERMANENT_SHARE_ERROR_CODES = new Set([
  "IMAGE_REJECTED_BY_MODERATION",
  "IMAGE_TOO_LARGE",
  "INVALID_IMAGE_FORMAT",
  "MODERATION_RATE_LIMITED",
  "QUOTA_EXCEEDED",
  "TOO_MANY_ACTIVE_SHARES",
  "USER_NOT_VERIFIED",
  "ACCOUNT_BANNED",
  // The core re-polls the share wait internally for up to 10 minutes before
  // surfacing UPLOAD_TIMEOUT, so by the time it reaches us it is final —
  // re-issuing the query would just start another 10-minute wait.
  "UPLOAD_TIMEOUT"
])

// Map error codes to translation keys for share instance errors
type ShareErrorKey =
  | "instances:_trn_share_errors.quota_exceeded"
  | "instances:_trn_share_errors.instance_too_large"
  | "instances:_trn_share_errors.not_enough_disk_space"
  | "instances:_trn_share_errors.too_many_shares"
  | "instances:_trn_share_errors.not_verified"
  | "instances:_trn_share_errors.network_error"
  | "instances:_trn_share_errors.upload_timeout"
  | "instances:_trn_share_errors.upload_failed"
  | "instances:_trn_share_errors.background_rejected"
  | "instances:_trn_share_errors.background_moderation_unavailable"
  | "instances:_trn_share_errors.background_rate_limited"
  | "instances:_trn_share_errors.background_too_large"
  | "instances:_trn_share_errors.background_invalid_format"

const getShareErrorKey = (code: string | null): ShareErrorKey => {
  switch (code) {
    case "QUOTA_EXCEEDED":
      return "instances:_trn_share_errors.quota_exceeded"
    case "INSTANCE_TOO_LARGE":
      return "instances:_trn_share_errors.instance_too_large"
    case "NOT_ENOUGH_DISK_SPACE":
      return "instances:_trn_share_errors.not_enough_disk_space"
    case "TOO_MANY_ACTIVE_SHARES":
      return "instances:_trn_share_errors.too_many_shares"
    case "USER_NOT_VERIFIED":
      return "instances:_trn_share_errors.not_verified"
    case "NETWORK_ERROR":
      return "instances:_trn_share_errors.network_error"
    case "UPLOAD_TIMEOUT":
      return "instances:_trn_share_errors.upload_timeout"
    case "IMAGE_REJECTED_BY_MODERATION":
      return "instances:_trn_share_errors.background_rejected"
    case "MODERATION_UNAVAILABLE":
      return "instances:_trn_share_errors.background_moderation_unavailable"
    case "MODERATION_RATE_LIMITED":
      return "instances:_trn_share_errors.background_rate_limited"
    case "IMAGE_TOO_LARGE":
      return "instances:_trn_share_errors.background_too_large"
    case "INVALID_IMAGE_FORMAT":
      return "instances:_trn_share_errors.background_invalid_format"
    default:
      return "instances:_trn_share_errors.upload_failed"
  }
}

interface ShareErrorDetails {
  totalBytes: number
  limitBytes: number
  largestFolders: { name: string; bytes: number }[]
}

interface ShareErrorState {
  code: string | null
  details?: ShareErrorDetails
}

interface Props {
  instanceId: number
}

/** What an entry holds, for the kinds a user would want to know about. */
const entryHint = (
  kind: FEShareEntryKind
): NamespacedTranslationKey | undefined => {
  switch (kind) {
    case "worlds":
      return "instances:_trn_instance_share.hint_worlds"
    case "backups":
      return "instances:_trn_instance_share.hint_backups"
    case "serverList":
      return "instances:_trn_instance_share.hint_server_list"
    case "screenshots":
      return "instances:_trn_instance_share.hint_screenshots"
    case "logs":
      return "instances:_trn_instance_share.hint_logs"
    default:
      return undefined
  }
}
/** Shared by default, though it can say something about its owner. */
const isPersonal = (kind: FEShareEntryKind) =>
  kind === "serverList" || kind === "screenshots" || kind === "logs"

const EXPIRATION_OPTIONS = [
  { value: "1", label: "_trn_instance_share.expiration_1d" },
  { value: "7", label: "_trn_instance_share.expiration_7d" }
  // { value: "30", label: "_trn_instance_share.expiration_30d" },
  // { value: "90", label: "_trn_instance_share.expiration_90d" }
] as const

function ShareInstance(props: ModalProps) {
  const data: () => Props = () => props.data

  const [t] = useTransContext()
  const modalsContext = useModal()
  const navigate = useNavigate()
  const globalStore = useGlobalStore()

  const isVerified = () => {
    const d = globalStore.gdlAccount.data
    return d?.status === "valid" && d.value.isEmailVerified
  }
  const [shareObject, setShareObject] =
    createSignal<FEWaitForInstanceShareResponse>()
  const [fileKey, setFileKey] = createSignal<string>()
  const [isLoading, setIsLoading] = createSignal(false)
  const [progress, setProgress] = createSignal(0)
  const [shareError, setShareError] = createSignal<ShareErrorState>()

  let sseStream: EventSource | null = null

  onCleanup(() => {
    if (sseStream) {
      sseStream.close()
      sseStream = null
    }
  })

  const [copyMode, setCopyMode] = createSignal<"code" | "link">("code")
  const copyDisplay = createMemo(() => {
    const obj = shareObject()
    if (!obj) return ""
    return copyMode() === "link"
      ? `https://.../${obj.share_code}`
      : obj.share_code
  })
  const copyValue = createMemo(() => {
    const obj = shareObject()
    if (!obj) return ""
    return copyMode() === "link"
      ? `https://gdl.gg/i/${obj.share_code}`
      : obj.share_code
  })

  // New state for title, expiration, and max downloads
  const [title, setTitle] = createSignal("")
  const [expirationDays, setExpirationDays] = createSignal("1")
  const [maxDownloads, setMaxDownloads] = createSignal<string>("")
  // The top-level files and folders of the instance, under Advanced for
  // those who want to choose. Worlds and their backups start unticked.
  const shareEntries = rspc.createQuery(() => ({
    queryKey: ["instance.getShareEntries", data()?.instanceId]
  }))
  const entries = () => shareEntries.data ?? []
  // What the user ticked or unticked; anything else keeps its default.
  const [chosen, setChosen] = createSignal<Record<string, boolean>>({})
  const isTicked = (entry: FEShareEntry) =>
    chosen()[entry.name] ?? entry.sharedByDefault

  const waitForShareInstanceMutation = rspc.createQuery(() => ({
    queryKey: [
      "instance.waitForShareInstance",
      { fileKey: fileKey()!, instanceId: data()?.instanceId }
    ],
    retry: (_failureCount, error) =>
      !PERMANENT_SHARE_ERROR_CODES.has(getErrorCode(error) ?? ""),
    enabled: !!fileKey()
  }))

  const handleShareError = (
    code: string | null,
    details?: ShareErrorDetails
  ) => {
    toast.error(t(getShareErrorKey(code)))
    setShareError({ code, details })
    setFileKey(undefined)
    setIsLoading(false)
    setProgress(0)
  }

  createEffect(() => {
    if (waitForShareInstanceMutation.data) {
      setShareObject(waitForShareInstanceMutation.data)
      setShareError(undefined)
      setIsLoading(false)
    }
  })

  createEffect(() => {
    const err = waitForShareInstanceMutation.error
    if (!err) return
    handleShareError(getErrorCode(err) ?? null)
  })

  const handleShare = async () => {
    if (isLoading()) return
    setShareError(undefined)
    setProgress(0)
    setIsLoading(true)

    // Build URL with new parameters
    const params = new URLSearchParams({
      instanceId: String(data()?.instanceId)
    })

    const titleValue = title().trim()
    if (titleValue) {
      params.set("title", titleValue)
    }

    params.set("expirationDays", expirationDays())

    const maxDownloadsValue = parseMaxDownloads(maxDownloads())
    if (maxDownloadsValue !== null) {
      params.set("maxDownloads", maxDownloadsValue.toString())
    }

    // Untouched, the core shares its default set: everything but worlds and
    // their backups. Once the user chose, it shares the ticked names only.
    if (shareEntries.data && Object.keys(chosen()).length > 0) {
      params.set(
        "entries",
        JSON.stringify(
          entries()
            .filter(isTicked)
            .map((entry) => entry.name)
        )
      )
    }

    // Once a terminal message (error or finished) arrives we close the stream
    // ourselves; this flag tells the native "error" listener below to ignore
    // the connection-closed event EventSource fires right after.
    let terminal = false
    const finish = () => {
      terminal = true
      sseStream?.close()
      sseStream = null
    }

    sseStream = new EventSource(
      apiUrl(`/instance/shareInstance?${params.toString()}`)
    )

    sseStream.onmessage = (event) => {
      let payload: {
        progress?: number
        finished?: string
        error?: {
          code?: string | null
          message?: string
          details?: ShareErrorDetails
        }
      }
      try {
        payload = JSON.parse(event.data)
      } catch {
        return
      }

      // Terminal failure. Share errors now arrive on the normal message stream
      // (not a named "error" event) so they can't collide with EventSource's
      // own dataless "error" event and get dropped.
      if (payload.error) {
        finish()
        handleShareError(payload.error.code ?? null, payload.error.details)
        return
      }

      if (typeof payload.progress === "number") {
        setProgress(payload.progress)
      }

      if (payload.finished) {
        finish()
        setFileKey(payload.finished)
      }
    }

    sseStream.addEventListener("error", () => {
      // Native EventSource connection error — carries no payload. If a terminal
      // message already arrived this is just the post-close signal; otherwise
      // the stream dropped before completing, so surface it as a failure
      // instead of silently resetting.
      if (terminal) return
      finish()
      handleShareError(null)
    })
  }

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <div class="w-120 flex flex-col gap-4">
        <Switch>
          <Match when={!isVerified()}>
            <VerificationRequiredPlaceholder />
          </Match>
          <Match when={!shareObject()}>
            <div class="text-lightSlate-50">
              <div class="mb-2 text-lg">
                <Trans key="instances:_trn_instance_share.title" />
              </div>
              <div class="text-lightSlate-500 mb-4 text-sm">
                <Trans key="instances:_trn_instance_share.description" />
              </div>

              <Show when={shareError()}>
                {(err) => (
                  <div class="border-red-500/40 bg-red-500/10 mb-4 rounded-md border p-3 text-sm">
                    <div class="text-red-400 flex items-center gap-2 font-medium">
                      <div class="i-ri:error-warning-line shrink-0" />
                      <span>{t(getShareErrorKey(err().code))}</span>
                    </div>
                    <Show
                      when={
                        err().code === "INSTANCE_TOO_LARGE" && err().details
                      }
                    >
                      {(details) => (
                        <div class="text-lightSlate-300 mt-2 flex flex-col gap-2">
                          <div>
                            {t("instances:_trn_share_errors.too_large_detail", {
                              size: formatBytes(details().totalBytes),
                              limit: formatBytes(details().limitBytes)
                            })}
                          </div>
                          <Show when={details().largestFolders.length > 0}>
                            <div>
                              <div class="text-lightSlate-400 mb-1 text-xs uppercase">
                                {t(
                                  "instances:_trn_share_errors.too_large_folders"
                                )}
                              </div>
                              <ul class="m-0 flex flex-col gap-1 p-0">
                                <For each={details().largestFolders}>
                                  {(folder) => (
                                    <li class="flex list-none justify-between gap-4">
                                      <span class="truncate font-mono">
                                        {folder.name}
                                      </span>
                                      <span class="text-lightSlate-400 shrink-0">
                                        {formatBytes(folder.bytes)}
                                      </span>
                                    </li>
                                  )}
                                </For>
                              </ul>
                            </div>
                          </Show>
                          <div class="text-lightSlate-400 text-xs">
                            {t("instances:_trn_share_errors.too_large_hint")}
                          </div>
                        </div>
                      )}
                    </Show>
                  </div>
                )}
              </Show>

              {/* Title Input */}
              <div class="mb-2">
                <label class="text-lightSlate-400 mb-1 block text-sm">
                  <Trans key="instances:_trn_instance_share.title_label" />
                </label>
                <Input
                  value={title()}
                  onInput={(e) => setTitle(e.currentTarget.value)}
                  placeholder={t(
                    "instances:_trn_instance_share.title_placeholder"
                  )}
                  disabled={isLoading()}
                  inputColor="bg-darkSlate-800"
                  class="w-full"
                />
              </div>

              {/* Advanced Options */}
              <Collapsable
                title={<Trans key="instances:_trn_instance_share.advanced" />}
                defaultOpened={false}
                noPadding
                size="small"
              >
                <div class="flex flex-col gap-4 py-2">
                  {/* Expiration Select */}
                  <div>
                    <label class="text-lightSlate-400 mb-1 block text-sm">
                      <Trans key="instances:_trn_instance_share.expiration_label" />
                    </label>
                    <Select
                      value={expirationDays()}
                      onChange={(val) => val && setExpirationDays(val)}
                      disabled={isLoading()}
                      options={EXPIRATION_OPTIONS.map((opt) => opt.value)}
                      itemComponent={(props) => {
                        const option = EXPIRATION_OPTIONS.find(
                          (opt) => opt.value === props.item.rawValue
                        )
                        return (
                          <SelectItem item={props.item}>
                            {option
                              ? t(`instances:${option.label}`)
                              : props.item.rawValue}
                          </SelectItem>
                        )
                      }}
                    >
                      <SelectTrigger class="w-full">
                        <SelectValue<string>>
                          {(state) => {
                            const option = EXPIRATION_OPTIONS.find(
                              (opt) => opt.value === state.selectedOption()
                            )
                            return option ? t(`instances:${option.label}`) : ""
                          }}
                        </SelectValue>
                      </SelectTrigger>
                      <SelectContent />
                    </Select>
                  </div>

                  {/* Max Downloads Input */}
                  <div>
                    <label class="text-lightSlate-400 mb-1 block text-sm">
                      <Trans key="instances:_trn_instance_share.max_downloads_label" />
                    </label>
                    <Input
                      type="number"
                      min="1"
                      max={MAX_DOWNLOADS_LIMIT}
                      value={maxDownloads()}
                      onInput={(e) => {
                        const validated = validateMaxDownloads(
                          e.currentTarget.value
                        )
                        setMaxDownloads(validated)
                        e.currentTarget.value = validated
                      }}
                      placeholder={t(
                        "instances:_trn_instance_share.max_downloads_placeholder"
                      )}
                      disabled={isLoading()}
                      inputColor="bg-darkSlate-800"
                      class="w-full"
                    />
                  </div>
                  {/* What is shared: for those who want to choose */}
                  <div>
                    <div class="text-lightSlate-400 mb-1 text-sm">
                      <Trans key="instances:_trn_instance_share.entries_label" />
                    </div>
                    <Show
                      when={!shareEntries.error}
                      fallback={
                        <div
                          class="text-red-400 text-sm"
                          data-testid="share-entries-failed"
                        >
                          <Trans key="instances:_trn_instance_share.entries_failed" />
                        </div>
                      }
                    >
                      <div
                        class="bg-darkSlate-800 flex max-h-64 min-h-10 flex-col gap-1 overflow-y-auto rounded-md px-3 py-2"
                        data-testid="share-entries"
                      >
                        <For each={entries()}>
                          {(entry) => (
                            <div
                              class="flex items-start gap-3 text-sm"
                              data-testid="share-entry"
                              data-entry={entry.name}
                            >
                              <Checkbox
                                checked={isTicked(entry)}
                                disabled={isLoading()}
                                ariaLabel={entry.name}
                                onChange={(checked) =>
                                  setChosen((current) => ({
                                    ...current,
                                    [entry.name]: checked
                                  }))
                                }
                              >
                                <span
                                  class="min-w-36 break-all font-mono"
                                  classList={{
                                    "text-lightSlate-50": isTicked(entry),
                                    "text-lightSlate-600": !isTicked(entry)
                                  }}
                                >
                                  {entry.name}
                                </span>
                              </Checkbox>
                              <Show when={entryHint(entry.kind)}>
                                {(hint) => (
                                  <span
                                    class="pt-0.5 text-xs"
                                    classList={{
                                      "text-yellow-400": isPersonal(entry.kind),
                                      "text-lightSlate-600": !isPersonal(
                                        entry.kind
                                      )
                                    }}
                                  >
                                    <Trans key={hint()} />
                                  </span>
                                )}
                              </Show>
                            </div>
                          )}
                        </For>
                      </div>
                    </Show>
                  </div>
                </div>
              </Collapsable>
            </div>
          </Match>
          <Match when={shareObject()}>
            <div class="text-lightSlate-50">
              <div class="mb-4 text-lg">
                <Trans key="instances:_trn_instance_share.share_code_label" />
              </div>
              <div class="text-lightSlate-500 mb-4 text-sm">
                <Trans key="instances:_trn_instance_share.share_code_details" />
              </div>
              <div class="flex items-stretch gap-2">
                <CopyText
                  size="large"
                  value={copyDisplay()}
                  copyValue={copyValue()}
                  onCopy={() =>
                    toast.success(t("general:_trn_general_copied_to_clipboard"))
                  }
                  class="!bg-darkSlate-800 flex-1 min-w-0"
                />
                <Select
                  class="flex shrink-0"
                  value={copyMode()}
                  onChange={(val) => val && setCopyMode(val)}
                  options={["code", "link"]}
                  itemComponent={(itemProps) => (
                    <SelectItem item={itemProps.item}>
                      {itemProps.item.rawValue === "code" ? "Code" : "Link"}
                    </SelectItem>
                  )}
                >
                  <SelectTrigger class="w-24 !h-full">
                    <SelectValue<string>>
                      {(state) =>
                        state.selectedOption() === "code" ? "Code" : "Link"
                      }
                    </SelectValue>
                  </SelectTrigger>
                  <SelectContent />
                </Select>
              </div>
              <div class="text-lightSlate-500 mt-4 text-sm">
                <Trans key="instances:_trn_instance_share.expires_at" />{" "}
                {new Date(shareObject()!.expires_at).toLocaleString()}
              </div>

              <div class="border-darkSlate-600 mt-4 flex items-center justify-end border-t pt-4">
                <Button
                  type="text"
                  size="small"
                  class="text-sm"
                  onClick={() => {
                    modalsContext?.closeModal()
                    navigate("/settings/accounts")
                  }}
                >
                  <div class="i-ri:settings-3-line" />
                  <Trans key="instances:_trn_instance_share.manage_shares" />
                  <div class="i-ri:arrow-right-line" />
                </Button>
              </div>
            </div>
          </Match>
        </Switch>

        <div class="flex justify-between gap-3">
          <Show when={!shareObject()}>
            <Button
              type="secondary"
              onClick={() => {
                if (sseStream) {
                  sseStream.close()
                  sseStream = null
                }
                modalsContext?.closeModal()
              }}
            >
              <Trans key="instances:_trn_instance_share.cancel" />
            </Button>
            <Button
              type="primary"
              disabled={isLoading()}
              onClick={handleShare}
              loading={!!fileKey() && isLoading()}
            >
              <Switch>
                <Match when={isLoading()}>
                  <div class="w-20">
                    <Progress color="bg-primary-400" value={progress()} />
                  </div>
                </Match>
                <Match when={!isLoading()}>
                  <div class="i-ri:share-line" />
                  <Trans key="instances:_trn_instance_share.share_button" />
                </Match>
              </Switch>
            </Button>
          </Show>
        </div>
      </div>
    </ModalLayout>
  )
}

export default ShareInstance
