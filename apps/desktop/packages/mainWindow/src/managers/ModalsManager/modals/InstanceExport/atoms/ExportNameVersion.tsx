import { useTransContext } from "@gd/i18n"
import { Input } from "@gd/ui"

export const ExportNameVersion = () => {
  const [t] = useTransContext()
  return (
    <div class="flex w-full flex-col gap-2">
      <span>{t("instance.instance_name")}</span>
      <Input
        inputColor="bg-darkSlate-900"
        icon={<div class="i-material-symbols:close" onClick={() => {}} />}
      />
      <span>{t("instance.version")}</span>
      <Input
        inputColor="bg-darkSlate-900"
        icon={<div class="i-material-symbols:close" onClick={() => {}} />}
      />
    </div>
  )
}
export default ExportNameVersion
