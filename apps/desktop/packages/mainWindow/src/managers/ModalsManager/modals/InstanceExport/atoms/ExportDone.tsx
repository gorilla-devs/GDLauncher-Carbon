import { useTransContext } from "@gd/i18n";
import { Button } from "@gd/ui";
import CheckIcon from "./CheckIcon";
import { ipcRenderer, shell } from "electron";

export default function ExportDone(props: { path: string }) {
  const [t] = useTransContext();
  return (
    <div class="flex gap-4 flex-col items-center justify-center h-full gap-2 p-4">
      <CheckIcon />
      <span>{`${t("instance.exported_to")} :`}</span>
      <div class="bg-[#1D2028] text-center w-full p-2 rounded-md">
        {props.path}
      </div>

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
    </div>
  );
}
