import { useTransContext } from "@gd/i18n";
import { Switch } from "@gd/ui";

const FilesBundle = () => {
  const [t] = useTransContext();
  return (
    <div class="w-full flex justify-between items-center pt-2">
      <span>{t("instance.bundle_files_text")}</span>
      <Switch />
    </div>
  );
};
export default FilesBundle;
