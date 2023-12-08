import { useModal } from "@/managers/ModalsManager";
import { setTaskId } from "@/utils/import";
import { rspc } from "@/utils/rspcClient";
import { useTransContext } from "@gd/i18n";
import { Button } from "@gd/ui";
import { setPayload, payload, setExportStep } from "..";
import { ExportArgs, ExportEntry } from "@gd/core_module/bindings";
import { instanceId } from "@/utils/browser";
import { buildNestedObject, checkedFiles } from "./ExportCheckboxParent";

const BeginExport = () => {
  const [t] = useTransContext();
  const modalsContext = useModal();
  const exportInstanceMutation = rspc.createMutation(["instance.export"], {
    onSuccess(taskId) {
      setTaskId(taskId);
      setExportStep(1);
    }
  });

  const validatePayload = (payload: ExportArgs) => {
    if (typeof payload.instance_id !== "number") return 0;
    if (typeof payload.save_path !== "string") return 0;
    return 1;
  };

  const handleExportInstance = () => {
    console.log(payload);
    setPayload((prev) => ({ ...prev, instance_id: instanceId() }));
    const obj = buildNestedObject(checkedFiles());
    setPayload((prev) => ({ ...prev, filter: { entries: obj } }));
    if (validatePayload(payload as ExportArgs)) {
      console.log("payload", payload);
      exportInstanceMutation.mutate({
        filter: payload.filter as ExportEntry,
        instance_id: payload.instance_id as number,
        save_path: payload.save_path as string,
        target: payload.target,
        link_mods: payload.link_mods
      });
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
      <Button onClick={handleExportInstance} type="primary" size="large">
        {t("instance.begin_export")}
      </Button>
    </div>
  );
};
export default BeginExport;
