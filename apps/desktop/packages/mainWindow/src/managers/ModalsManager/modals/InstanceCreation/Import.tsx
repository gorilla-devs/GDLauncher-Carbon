import { useModal } from "../..";
import ExportDone from "../InstanceExport/ExportDone";
import Exporting from "../InstanceExport/Exporting";
import ThirdStep from "../OnBoarding/ThirdStep";

const Import = () => {
  const modalsContext = useModal();
  return (
    <ThirdStep
      prevStep={() => {
        modalsContext?.closeModal();
      }}
      isImportInstance={true}
    />
  );
};

export default Import;
