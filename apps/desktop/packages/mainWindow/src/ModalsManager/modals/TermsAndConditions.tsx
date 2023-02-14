/* eslint-disable i18next/no-literal-string */
import { ModalProps } from "..";
import ModalLayout from "../ModalLayout";

const TermsAndConditions = (props: ModalProps) => {
  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title} onClose={}>
      <div class="h-130 w-190">TermsAndConditions</div>
    </ModalLayout>
  );
};

export default TermsAndConditions;
