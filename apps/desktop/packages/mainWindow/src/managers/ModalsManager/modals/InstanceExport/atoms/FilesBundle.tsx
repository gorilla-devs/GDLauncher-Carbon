import { useTransContext } from "@gd/i18n";
import { Switch } from "@gd/ui";
import { createSignal } from "solid-js";
import { setPayload, payload } from "..";

const FilesBundle = () => {
  const [t] = useTransContext();
  const handleSwitch = () => {
    setPayload({ ...payload, links_mod: !payload.links_mod });
  };
  return (
    <div class="w-full flex justify-between items-center pt-2">
      <span>{t("instance.bundle_files_text")}</span>
      <Switch onClick={handleSwitch} checked={payload.links_mod} />
    </div>
  );
};
export default FilesBundle;
