import { useTransContext } from "@gd/i18n"
import { Button } from "@gd/ui"
import CheckIcon from "./CheckIcon"
import { Show } from "solid-js"
import { failedMsg } from "./Exporting"

export default function ExportDone(props: { path: string }) {
  const [t] = useTransContext()
  return (
    <div class="flex h-full flex-col items-center justify-center gap-2 gap-4 p-4">
      <Show when={!failedMsg()}>
        <CheckIcon />
        <span>{`${t("instances:_trn_exported_to")} :`}</span>
      </Show>

      <div class="w-full rounded-md bg-[#1D2028] p-2 text-center leading-10">
        {failedMsg() ? failedMsg() : props.path}
      </div>

      <Show when={!failedMsg()}>
        <Button
          style={{ width: "100%", "max-width": "100%" }}
          type="primary"
          onClick={async () => {
            window.openFolder(props.path)
          }}
        >
          <div class="i-hugeicons:folder-open" />
          <span>{t("instances:_trn_open_folder")}</span>
        </Button>
      </Show>
    </div>
  )
}
