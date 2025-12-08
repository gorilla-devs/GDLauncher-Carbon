import { useTransContext } from "@gd/i18n"
import { Input } from "@gd/ui"
import { setPayload, payload } from ".."

const ExportVersion = () => {
  const [t] = useTransContext()

  return (
    <div class="flex w-full items-center justify-between pt-4">
      <div class="flex items-center gap-2">
        <div>{t("instances:_trn_export_version")}</div>
      </div>
      <Input
        class="w-32"
        value={payload.version}
        onInput={(e) => {
          setPayload("version", e.currentTarget.value)
        }}
        placeholder="1.0.0"
      />
    </div>
  )
}

export default ExportVersion
