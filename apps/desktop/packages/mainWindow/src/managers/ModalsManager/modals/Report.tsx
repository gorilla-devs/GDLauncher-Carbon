import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import {
  Button,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  TextArea,
  toast
} from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { createSignal, Show } from "solid-js"
import { rspc } from "@/utils/rspcClient"
import { getErrorCode } from "@/components/SharePreviewContent"

/**
 * Generic report dialog. Reusable across report target types (shares today,
 * users and other entities tomorrow). Callers open it with `openModal("report", data)`
 * where `data` is a `ReportModalData` — see below.
 */

export interface ReportTarget {
  kind: "share"
  shareCode: string
  /** Optional label (e.g., share title) shown in the dialog header */
  displayName?: string
}

export interface ReportModalData {
  target: ReportTarget
  /** If set, the type selector is hidden and this value is used. */
  defaultReportType?: string
  /** Invoked after a successful report submission. */
  onReported?: () => void
}

/** Report-type options available for each target kind. */
const OPTIONS_BY_KIND = {
  share: ["share_background", "share_title", "share_content"] as const
} satisfies Record<ReportTarget["kind"], readonly string[]>

type ReportOptionValue =
  (typeof OPTIONS_BY_KIND)[keyof typeof OPTIONS_BY_KIND][number]

type ReportOptionLabelKey =
  | "instances:_trn_report.option_share_background"
  | "instances:_trn_report.option_share_title"
  | "instances:_trn_report.option_share_content"

const optionLabelKey = (value: string): ReportOptionLabelKey | null => {
  switch (value as ReportOptionValue) {
    case "share_background":
      return "instances:_trn_report.option_share_background"
    case "share_title":
      return "instances:_trn_report.option_share_title"
    case "share_content":
      return "instances:_trn_report.option_share_content"
    default:
      return null
  }
}

const MAX_REASON_LENGTH = 1000

const Report = (props: ModalProps) => {
  const data: () => ReportModalData | undefined = () => props.data
  const [t] = useTransContext()
  const modalsContext = useModal()

  const initialType = () => {
    const d = data()
    if (!d) return ""
    if (d.defaultReportType) return d.defaultReportType
    const options = OPTIONS_BY_KIND[d.target.kind]
    return options[0] ?? ""
  }

  const [reportType, setReportType] = createSignal<string>(initialType())
  const [reason, setReason] = createSignal("")

  const reportShareMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.reportShare"]
  }))

  const isPending = () => reportShareMutation.isPending

  const submit = async () => {
    const d = data()
    if (!d) return

    const payloadReason = reason().trim() || null
    try {
      if (d.target.kind === "share") {
        await reportShareMutation.mutateAsync({
          shareCode: d.target.shareCode,
          reportType: reportType(),
          reason: payloadReason
        })
      } else {
        // Exhaustiveness guard for future kinds.
        const _never: never = d.target.kind
        throw new Error(`Unsupported report target: ${String(_never)}`)
      }

      toast.success(t("instances:_trn_report.submitted"))
      d.onReported?.()
      modalsContext?.closeModal()
    } catch (err) {
      const code = getErrorCode(err)
      switch (code) {
        case "TOO_MANY_REQUESTS":
          toast.error(t("instances:_trn_report.rate_limited"))
          break
        case "SHARE_NOT_FOUND":
          toast.error(t("instances:_trn_report.target_not_found"))
          break
        default:
          toast.error(t("instances:_trn_report.submit_failed"))
      }
    }
  }

  const typeOptions = (): string[] => {
    const d = data()
    if (!d) return []
    return [...OPTIONS_BY_KIND[d.target.kind]]
  }

  const showTypeSelector = () => !data()?.defaultReportType

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <div class="w-100 flex flex-col gap-4">
        <p class="text-lightSlate-300 text-sm">
          <Trans key="instances:_trn_report.description" />
        </p>

        <Show
          when={data()?.target.kind === "share" && data()?.target.displayName}
        >
          <div class="bg-darkSlate-800 text-lightSlate-200 rounded px-3 py-2 text-sm">
            {data()!.target.displayName}
          </div>
        </Show>

        <Show when={showTypeSelector()}>
          <div>
            <label class="text-lightSlate-400 mb-1 block text-sm">
              <Trans key="instances:_trn_report.type_label" />
            </label>
            <Select
              value={reportType()}
              onChange={(val) => val && setReportType(val)}
              options={typeOptions()}
              itemComponent={(itemProps) => {
                const key = optionLabelKey(itemProps.item.rawValue)
                return (
                  <SelectItem item={itemProps.item}>
                    {key ? t(key) : itemProps.item.rawValue}
                  </SelectItem>
                )
              }}
            >
              <SelectTrigger class="w-full">
                <SelectValue<string>>
                  {(state) => {
                    const key = optionLabelKey(state.selectedOption())
                    return key ? t(key) : ""
                  }}
                </SelectValue>
              </SelectTrigger>
              <SelectContent />
            </Select>
          </div>
        </Show>

        <div>
          <label class="text-lightSlate-400 mb-1 block text-sm">
            <Trans key="instances:_trn_report.reason_label" />
          </label>
          <TextArea
            value={reason()}
            placeholder={t("instances:_trn_report.reason_placeholder")}
            class="w-full"
            inputClass="bg-darkSlate-800 min-h-24"
            onInput={(e: { currentTarget: HTMLTextAreaElement }) => {
              const trimmed = e.currentTarget.value.slice(0, MAX_REASON_LENGTH)
              setReason(trimmed)
              e.currentTarget.value = trimmed
            }}
          />
          <div class="text-lightSlate-500 mt-1 text-right text-xs">
            {reason().length}/{MAX_REASON_LENGTH}
          </div>
        </div>

        <div class="flex justify-between">
          <Button
            type="secondary"
            onClick={() => modalsContext?.closeModal()}
            disabled={isPending()}
          >
            <Trans key="instances:_trn_report.cancel" />
          </Button>
          <Button type="primary" onClick={submit} loading={isPending()}>
            <Trans key="instances:_trn_report.submit" />
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default Report
