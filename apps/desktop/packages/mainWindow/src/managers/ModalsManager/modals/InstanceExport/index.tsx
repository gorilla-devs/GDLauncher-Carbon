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

const InstanceExport = (props: ModalProps) => {
  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      overflowHiddenDisabled={true}
      noPadding={true}
      height="h-96"
    >
      <div class="flex flex-col p-4 scrollbar-hide overflow-y-scroll w-120  ">
        <ExportFormat />
        <ExportNameVersion />
        <FilesSelection />
        <FilesBundle />
        <ExportPath />
        <BeginExport />
      </div>
    </ModalLayout>
  );
};

export default InstanceExport;
