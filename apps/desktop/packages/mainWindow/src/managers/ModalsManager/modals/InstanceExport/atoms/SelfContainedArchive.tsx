import { useTransContext } from "@gd/i18n"
import { Switch, Tooltip, TooltipContent, TooltipTrigger, AnimatedIcon } from "@gd/ui"
import { setPayload, payload } from ".."

const SelfContainedArchive = () => {
  const [t] = useTransContext()
  const handleSwitch = () => {
    setPayload({
      ...payload,
      self_contained_addons_bundling: !payload.self_contained_addons_bundling
    })
  }
  return (
    <div class="flex w-full items-center justify-between pt-4">
      <div class="flex items-center gap-2">
        <div>{t("instance.self_contained_addons_bundling")}</div>
        <Tooltip>
          <TooltipTrigger>
            <AnimatedIcon icon="i-hugeicons:information-circle" class="text-darkSlate-400 hover:text-lightSlate-50 cursor-pointer transition-color duration-100 ease-in-out" size="text-2xl" />
          </TooltipTrigger>
          <TooltipContent>
            {t("instance.self_contained_addons_bundling_tooltip")}
          </TooltipContent>
        </Tooltip>
      </div>
      <Switch
        onChange={handleSwitch}
        checked={payload.self_contained_addons_bundling}
      />
    </div>
  )
}
export default SelfContainedArchive
