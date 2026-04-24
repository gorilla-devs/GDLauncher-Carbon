import { toast } from "@gd/ui"
import { useTransContext } from "@gd/i18n"
import { Show } from "solid-js"

interface CopyableFieldProps {
  label?: string
  value?: string | null
}

const CopyableField = (props: CopyableFieldProps) => {
  const [t] = useTransContext()

  const handleCopy = () => {
    if (props.value) {
      navigator.clipboard.writeText(props.value)
      toast.success(t("general:_trn_general_copied_to_clipboard"))
    }
  }

  return (
    <div
      class="group flex cursor-pointer items-center gap-2 text-lightSlate-300 hover:text-lightSlate-50"
      onClick={handleCopy}
    >
      <Show when={props.label}>
        <span class="text-lightSlate-500 text-sm">{props.label}:</span>
      </Show>
      <span class="font-mono text-sm">{props.value || "-"}</span>
      <div class="i-hugeicons:clipboard text-sm opacity-0 transition-opacity group-hover:opacity-100" />
    </div>
  )
}

export default CopyableField
