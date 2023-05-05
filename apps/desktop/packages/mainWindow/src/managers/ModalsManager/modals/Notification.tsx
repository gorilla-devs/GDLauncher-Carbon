/* eslint-disable i18next/no-literal-string */
import { ModalProps } from "..";
import ModalLayout from "../ModalLayout";

const Notification = (props: ModalProps) => {
  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <div class="h-130 w-190 overflow-hidden">Notification</div>
    </ModalLayout>
  );
};

export default Notification;
