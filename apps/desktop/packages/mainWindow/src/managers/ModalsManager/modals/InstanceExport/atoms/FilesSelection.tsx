import { useTransContext } from "@gd/i18n"
import ExportCheckboxParent from "./ExportCheckboxParent"

interface Props {
  instanceId: number
}

const FilesSelection = (props: Props) => {
  const [t] = useTransContext()
  return (
    <div class="w-full flex flex-col gap-2 pt-2">
      <span>{t("instance.select_files_text")}</span>
      <div class="w-full rounded-md bg-darkSlate-900 overflow-y-scroll h-44">
        <ExportCheckboxParent instanceId={props.instanceId} />
      </div>
    </div>
  )
}
export default FilesSelection
