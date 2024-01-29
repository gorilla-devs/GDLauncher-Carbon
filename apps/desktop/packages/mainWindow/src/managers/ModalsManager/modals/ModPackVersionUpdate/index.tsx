import { Select } from "@gd/ui";
import { ModalProps } from "../..";
import ModalLayout from "../../ModalLayout";

const ModPackVersionUpdate = (props: ModalProps) => {
  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      overflowHiddenDisabled={true}
      noPadding={true}
      scrollable="overflow-y-scroll scrollbar-hide"
      // height="h-96"
    >
      <div class="flex flex-col p-4 w-120"></div>
    </ModalLayout>
  );
};

export default ModPackVersionUpdate;
