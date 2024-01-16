import { useTransContext } from "@gd/i18n";
import ExportCheckboxParent from "./ExportCheckboxParent";

const FilesSelection = () => {
  const [t] = useTransContext();
  return (
    <div class="w-full flex flex-col gap-2 pt-2">
      <span>{t("instance.select_files_text")}</span>
      <div class="w-full rounded-md bg-darkSlate-900 h-44 overflow-y-scroll">
        <ExportCheckboxParent />
      </div>
    </div>
  );
};
export default FilesSelection;
