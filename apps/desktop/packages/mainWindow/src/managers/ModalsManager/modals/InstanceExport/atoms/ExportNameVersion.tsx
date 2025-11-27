import { useTransContext } from "@gd/i18n"
import { Input } from "@gd/ui"

export const ExportNameVersion = () => {
  const [t] = useTransContext()
  return (
    <div class="flex w-full flex-col gap-2">
      <span>{t("instances:_trn_instance_name")}</span>
      <Input
        inputColor="bg-darkSlate-900"
        icon={<div class="i-material-symbols:close" onClick={() => {}} />}
      />
      <span>{t("instances:_trn_version")}</span>
      <Input
        inputColor="bg-darkSlate-900"
        icon={<div class="i-material-symbols:close" onClick={() => {}} />}
      />
    </div>
  )
}
export default ExportNameVersion
