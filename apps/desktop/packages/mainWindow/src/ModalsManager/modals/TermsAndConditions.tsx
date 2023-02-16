/* eslint-disable i18next/no-literal-string */
import { ModalProps, useModal } from "..";
import ModalLayout from "../ModalLayout";

const TermsAndConditions = (props: ModalProps) => {
  const modalsManager = useModal();
  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <div class="h-130 w-190">
        <button
          onClick={() => {
            modalsManager?.openModal("privacyPolicy");
          }}
        >
          OPEN
        </button>
      </div>
    </ModalLayout>
  );
};

export default TermsAndConditions;
