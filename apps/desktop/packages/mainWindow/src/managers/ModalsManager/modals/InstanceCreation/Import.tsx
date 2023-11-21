import { useModal } from "../..";
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
