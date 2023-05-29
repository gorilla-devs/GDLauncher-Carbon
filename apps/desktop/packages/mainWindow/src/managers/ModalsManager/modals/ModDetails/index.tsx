/* eslint-disable i18next/no-literal-string */
import { ModalProps } from "../..";
import ModalLayout from "../../ModalLayout";

const AcceptableUsePolicy = (props: ModalProps) => {
  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <div class="h-130 w-190">Mod Details</div>
    </ModalLayout>
  );
};

export default AcceptableUsePolicy;
