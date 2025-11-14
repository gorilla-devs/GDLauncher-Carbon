import { useTransContext } from "@gd/i18n"
import ExportCheckboxParent from "./ExportCheckboxParent"

interface Props {
  instanceId: number
}

const FilesSelection = (props: Props) => {
  const [t] = useTransContext()
  return (
    <div class="flex w-full flex-col gap-2 pt-2">
      <span>{t("instances:_trn_select_files_text")}</span>
      <div class="bg-darkSlate-900 h-44 w-full overflow-y-scroll rounded-md">
        <ExportCheckboxParent instanceId={props.instanceId} />
      </div>
    </div>
  )
}
export default FilesSelection
