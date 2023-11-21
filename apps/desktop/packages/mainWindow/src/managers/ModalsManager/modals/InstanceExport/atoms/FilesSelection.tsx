import { useTransContext } from "@gd/i18n";

const FilesSelection = () => {
  const [t] = useTransContext();
  return (
    <div class="w-full flex flex-col gap-2 pt-2">
      <span>{t("instance.select_files_text")}</span>
      <div class="w-full rounded-md bg-darkSlate-900 h-10"></div>
    </div>
  );
};
export default FilesSelection;
