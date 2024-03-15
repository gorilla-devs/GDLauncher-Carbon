import { useTransContext } from "@gd/i18n";
import { Switch, Tooltip } from "@gd/ui";
import { setPayload, payload } from "..";

const SelfContainedArchive = () => {
  const [t] = useTransContext();
  const handleSwitch = () => {
    setPayload({
      ...payload,
      self_contained_addons_bundling: !payload.self_contained_addons_bundling
    });
  };
  return (
    <div class="w-full flex justify-between items-center pt-4">
      <div class="flex items-center gap-2">
        <div>{t("instance.self_contained_addons_bundling")}</div>
        <Tooltip content={t("instance.self_contained_addons_bundling_tooltip")}>
          <div class="text-2xl text-darkSlate-400 duration-100 ease-in-out cursor-pointer i-ri:information-fill transition-color hover:text-white" />
        </Tooltip>
      </div>
      <Switch
        onChange={handleSwitch}
        checked={payload.self_contained_addons_bundling}
      />
    </div>
  );
};
export default SelfContainedArchive;
