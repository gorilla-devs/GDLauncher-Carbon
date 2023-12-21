import { useTransContext } from "@gd/i18n";
import { Input } from "@gd/ui";

export const ExportNameVersion = () => {
  const [t] = useTransContext();
  return (
    <div class="w-full flex flex-col  gap-2">
      <span>{t("instance.instance_name")}</span>
      <Input
        inputColor="bg-darkSlate-900"
        icon={<div onClick={() => {}} class="i-material-symbols:close"></div>}
      />
      <span>{t("instance.version")}</span>
      <Input
        inputColor="bg-darkSlate-900"
        icon={<div onClick={() => {}} class="i-material-symbols:close"></div>}
      />
    </div>
  );
};
export default ExportNameVersion;
