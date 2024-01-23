import { useTransContext } from "@gd/i18n";
import { Button } from "@gd/ui";
import CheckIcon from "./CheckIcon";
import { Show } from "solid-js";
import { failedMsg } from "./Exporting";

export default function ExportDone(props: { path: string }) {
  const [t] = useTransContext();
  return (
    <div class="flex gap-4 flex-col items-center justify-center h-full gap-2 p-4">
      <Show when={!failedMsg()}>
        <CheckIcon />
        <span>{`${t("instance.exported_to")} :`}</span>
      </Show>

      <div class="bg-[#1D2028] text-center w-full p-2 rounded-md leading-10">
        {failedMsg() ? failedMsg() : props.path}
      </div>

      <Show when={!failedMsg()}>
        <Button
          style={{ width: "100%", "max-width": "100%" }}
          type="primary"
          onClick={async () => {
            window.openFolder(props.path);
          }}
        >
          <div class="i-ri:folder-line" />
          <span>{t("instance.open_folder")}</span>
        </Button>
      </Show>
    </div>
  );
}
