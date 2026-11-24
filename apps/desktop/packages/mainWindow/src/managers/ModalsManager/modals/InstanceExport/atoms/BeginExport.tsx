import { useModal } from "@/managers/ModalsManager"
import { setTaskId } from "@/utils/import"
import { rspc } from "@/utils/rspcClient"
import { useTransContext } from "@gd/i18n"
import { Button } from "@gd/ui"
import { CancelLink, DialogFooter } from "../../../DialogFooter"
import { setPayload, payload, setExportStep } from ".."
import { ExportArgs, ExportEntry } from "@gd/core_module/bindings"
import {
  buildNestedObject,
  checkedFiles,
  type NestedEntry
} from "./ExportCheckboxParent"
import { validArchivePath } from "./archivePath"
import { setFailedMsg } from "./Exporting"

function convertNestedObject(obj: NestedEntry): ExportEntry {
  const result: Record<string, ExportEntry | null> = {}

  if (obj.entries) {
    for (const key in obj.entries) {
      const value = obj.entries[key]
      if (value && value.entries !== null) {
        result[key] = convertNestedObject(value)
      } else {
        result[key] = null
      }
    }
  }

  return { entries: result }
}

interface Props {
  instanceId: number
}

const BeginExport = (props: Props) => {
  const [t] = useTransContext()
  const modalsContext = useModal()
  const exportInstanceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.export"],
    onSuccess(taskId) {
      setTaskId(taskId)
      setExportStep(1)
    }
  }))

  const validatePayload = (payload: ExportArgs) => {
    if (typeof payload.instance_id !== "number") return false
    return validArchivePath(payload.save_path, payload.target)
  }

  const handleExportInstance = () => {
    if (!validatePayload(payload as ExportArgs)) return
    setPayload((prev) => ({
      ...prev,
      instance_id: props.instanceId
    }))
    setFailedMsg(undefined)
    const obj = buildNestedObject(checkedFiles())
    const converted = convertNestedObject({ entries: obj })
    setPayload((prev) => ({ ...prev, filter: converted }))

    const exportObj = {
      filter: payload.filter as ExportEntry,
      instance_id: payload.instance_id!,
      save_path: payload.save_path!,
      target: payload.target,
      self_contained_addons_bundling: payload.self_contained_addons_bundling,
      version: payload.version
    }

    if (validatePayload(exportObj)) {
      exportInstanceMutation.mutate(exportObj)
    }
  }

  return (
    <DialogFooter class="w-full pt-4">
      <CancelLink onClick={() => modalsContext?.closeModal()} />
      <Button
        onClick={() => {
          handleExportInstance()
        }}
        type="primary"
        size="large"
        disabled={!validatePayload(payload as ExportArgs)}
      >
        {t("instances:_trn_begin_export")}
      </Button>
    </DialogFooter>
  )
}
export default BeginExport
