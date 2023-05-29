import { Trans } from "@gd/i18n";
import { ModalProps } from "@/managers/ModalsManager";
import ModalLayout from "@/managers/ModalsManager/ModalLayout";
import { Button, Input } from "@gd/ui";

const AddMod = (props: ModalProps) => {
  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <div class="h-130 w-190">Add Mod</div>
    </ModalLayout>
  );
};

export default AddMod;
