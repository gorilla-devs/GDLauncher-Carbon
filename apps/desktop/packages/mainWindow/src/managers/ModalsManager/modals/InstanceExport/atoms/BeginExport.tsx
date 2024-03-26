import { useModal } from "@/managers/ModalsManager";
import { setTaskId } from "@/utils/import";
import { rspc } from "@/utils/rspcClient";
import { useTransContext } from "@gd/i18n";
import { Button } from "@gd/ui";
import { setPayload, payload, setExportStep } from "..";
import { ExportArgs, ExportEntry } from "@gd/core_module/bindings";
import { instanceId } from "@/utils/browser";
import { buildNestedObject, checkedFiles } from "./ExportCheckboxParent";
import _ from "lodash";
import { setFailedMsg } from "./Exporting";

function convertNestedObject(obj: any): any {
  const result: any = {};

  for (const key in obj.entries) {
    if (key in obj.entries) {
      const value = obj.entries[key];
      if (value && typeof value === "object" && value.entries !== null) {
        // If the current value has a nested 'entries' object, recursively process it
        result[key] = convertNestedObject(value as any);
      } else {
        // If 'entries' is null or not an object, set the key's value to null
        result[key] = null;
      }
    }
  }

  return { entries: result };
}

const BeginExport = () => {
  const [t] = useTransContext();
  const modalsContext = useModal();
  const exportInstanceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.export"],
    onSuccess(taskId) {
      setTaskId(taskId);
      setExportStep(1);
    }
  }));

  const validatePayload = (payload: ExportArgs) => {
    if (typeof payload.instance_id !== "number") return false;
    if (typeof payload.save_path !== "string") return false;
    const extension = _.last(payload.save_path.split("."));
    if (extension !== "zip" && extension !== "mrpack") return false;
    return true;
  };

  const handleExportInstance = () => {
    setPayload((prev) => ({ ...prev, instance_id: instanceId() }));
    setFailedMsg(undefined);
    const obj = buildNestedObject(checkedFiles());
    const converted = convertNestedObject({ entries: obj });
    setPayload((prev) => ({ ...prev, filter: converted }));

    const exportObj = {
      filter: payload.filter as ExportEntry,
      instance_id: payload.instance_id as number,
      save_path: payload.save_path as string,
      target: payload.target,
      self_contained_addons_bundling: payload.self_contained_addons_bundling
    };

    if (validatePayload(exportObj)) {
      exportInstanceMutation.mutate(exportObj);
    }
  };

  return (
    <div class="flex justify-between items-center w-full pt-4">
      <Button
        type="secondary"
        size="large"
        onClick={() => {
          modalsContext?.closeModal();
        }}
      >
        {t("instance.cancel_export")}
      </Button>
      <Button
        onClick={() => {
          handleExportInstance();
        }}
        type="primary"
        size="large"
        disabled={!validatePayload(payload as ExportArgs)}
      >
        {t("instance.begin_export")}
      </Button>
    </div>
  );
};
export default BeginExport;
