/* eslint-disable i18next/no-literal-string */
import { ModalProps } from "../..";
import ModalLayout from "../../ModalLayout";
import { FEMod } from "@gd/core_module/bindings";

const ModDetails = (props: ModalProps) => {
  const modDetails = () => props.data?.mod as FEMod;

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <div class="h-130 w-190">{modDetails()?.name}</div>
    </ModalLayout>
  );
};

export default ModDetails;
