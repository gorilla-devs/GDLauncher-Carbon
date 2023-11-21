import { Tab, TabList, TabPanel, Tabs } from "@gd/ui";
import { ModalProps } from "../..";
import ModalLayout from "../../ModalLayout";
import { Trans } from "@gd/i18n";
import ExportFormat from "./ExportFormat";

const InstanceExport = (props: ModalProps) => {
  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      overflowHiddenDisabled={true}
      noPadding={true}
    >
      <div class="flex flex-col justify-between scrollbar-hide overflow-y-scroll w-120 h-full">
        <ExportFormat />
      </div>
    </ModalLayout>
  );
};

export default InstanceExport;
