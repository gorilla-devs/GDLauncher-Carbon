import { Tab, TabList, TabPanel, Tabs } from "@gd/ui";
import { ModalProps } from "../..";
import ModalLayout from "../../ModalLayout";
import { Trans } from "@gd/i18n";
import ExportFormat from "./atoms/ExportFormat";
import ExportNameVersion from "./atoms/ExportNameVersion";
import FilesSelection from "./atoms/FilesSelection";
import FilesBundle from "./atoms/FilesBundle";
import ExportPath from "./atoms/ExportPath";
import BeginExport from "./atoms/BeginExport";
import { createSignal } from "solid-js";
import { ExportTarget } from "@gd/core_module/bindings";
import { createStore } from "solid-js/store";

interface IPayload {
  instance_id: string | undefined;
  target: ExportTarget;
  save_path: string | undefined;
  links_mod: boolean;
  filter: {};
}

const [payload, setPayload] = createStore<IPayload>({
  instance_id: undefined,
  target: "Curseforge",
  save_path: undefined,
  links_mod: false,
  filter: {}
});
export { payload, setPayload };
const InstanceExport = (props: ModalProps) => {
  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      overflowHiddenDisabled={true}
      noPadding={true}
      scrollable="overflow-y-scroll scrollbar-hide"
      // height="h-96"
    >
      <div class="flex flex-col p-4 w-120 ">
        <ExportFormat />
        {/* <ExportNameVersion /> */}
        <FilesSelection />
        <FilesBundle />
        <ExportPath />
        <BeginExport />
      </div>
    </ModalLayout>
  );
};

export default InstanceExport;
