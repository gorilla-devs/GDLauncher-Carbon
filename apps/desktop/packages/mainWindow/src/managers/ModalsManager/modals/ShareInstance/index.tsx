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
  Collapsable
} from "@gd/ui"
import { ModalProps, useModal } from "../.."
import ModalLayout from "../../ModalLayout"
import { port, rspc } from "@/utils/rspcClient"
import {
  createEffect,
  createSignal,
  Match,
  onCleanup,
  Show,
  Switch
} from "solid-js"
import { FEWaitForInstanceShareResponse } from "@gd/core_module/bindings"
import { Trans, useTransContext } from "@gd/i18n"
import { useNavigate } from "@solidjs/router"
import { MAX_DOWNLOADS_LIMIT, validateMaxDownloads } from "@/utils/validation"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import VerificationRequiredPlaceholder from "@/components/VerificationRequiredPlaceholder"

// Map error codes to translation keys for share instance errors
type ShareErrorKey =
  | "instances:_trn_share_errors.quota_exceeded"
  | "instances:_trn_share_errors.too_many_shares"
  | "instances:_trn_share_errors.not_verified"
  | "instances:_trn_share_errors.network_error"
  | "instances:_trn_share_errors.upload_timeout"
  | "instances:_trn_share_errors.upload_failed"

const getShareErrorKey = (code: string | null): ShareErrorKey => {
  switch (code) {
    case "QUOTA_EXCEEDED":
      return "instances:_trn_share_errors.quota_exceeded"
    case "TOO_MANY_ACTIVE_SHARES":
      return "instances:_trn_share_errors.too_many_shares"
    case "USER_NOT_VERIFIED":
      return "instances:_trn_share_errors.not_verified"
    case "NETWORK_ERROR":
      return "instances:_trn_share_errors.network_error"
    case "UPLOAD_TIMEOUT":
      return "instances:_trn_share_errors.upload_timeout"
    default:
      return "instances:_trn_share_errors.upload_failed"
  }
}

interface Props {
  instanceId: number
}

const EXPIRATION_OPTIONS = [
  { value: "1", label: "_trn_instance_share.expiration_1d" },
  { value: "7", label: "_trn_instance_share.expiration_7d" },
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

  let sseStream: EventSource | null = null

  onCleanup(() => {
    if (sseStream) {
      sseStream.close()
      sseStream = null
    }
  })

  // New state for title, expiration, and max downloads
  const [title, setTitle] = createSignal("")
  const [expirationDays, setExpirationDays] = createSignal("1")
  const [maxDownloads, setMaxDownloads] = createSignal<string>("")

  const waitForShareInstanceMutation = rspc.createQuery(() => ({
    queryKey: [
      "instance.waitForShareInstance",
      { fileKey: fileKey()!, instanceId: data()?.instanceId }
    ],
    retry: true,
    enabled: !!fileKey()
  }))

  createEffect(() => {
    if (waitForShareInstanceMutation.data) {
      setShareObject(waitForShareInstanceMutation.data)
      setIsLoading(false)
    }
  })

  const handleShare = async () => {
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

    const maxDownloadsValue = maxDownloads().trim()
    if (maxDownloadsValue && parseInt(maxDownloadsValue) >= 1) {
      params.set("maxDownloads", maxDownloadsValue)
    }

    sseStream = new EventSource(
      `http://127.0.0.1:${port}/instance/shareInstance?${params.toString()}`
    )

    sseStream.onmessage = (event) => {
      const data = JSON.parse(event.data)
      console.log("data", data)
      if (data.progress) {
        setProgress(data.progress)
      }
      if (data.finished) {
        setFileKey(data.finished)
        sseStream.close()
      }
    }

    sseStream.addEventListener("error", (event) => {
      console.log("error", event)
      try {
        const data = JSON.parse((event as MessageEvent).data)
        // New format: { error: { code: string, message: string } }
        const errorCode = data?.error?.code || null
        toast.error(t(getShareErrorKey(errorCode)))
      } catch {
        // Fallback for unparseable errors
        toast.error(t("instances:_trn_share_errors.upload_failed"))
      }
      setIsLoading(false)
      sseStream.close()
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
              <CopyText
                size="large"
                value={shareObject()!.share_code}
                onCopy={() =>
                  toast.success(t("general:_trn_general_copied_to_clipboard"))
                }
                class="!bg-darkSlate-800"
              />
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
