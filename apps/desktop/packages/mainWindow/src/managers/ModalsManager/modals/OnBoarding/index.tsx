import ModalLayout from "../../ModalLayout";
import { ModalProps } from "../..";
import { Steps } from "@gd/ui";
import { Match, Switch, createSignal } from "solid-js";
import SecondStep from "./SecondStep";
import ThirdStep from "./ThirdStep";
import FirstStep from "./FirstStep";
import { useTransContext } from "@gd/i18n";

const OnBoarding = (props: ModalProps) => {
  const [t] = useTransContext();

  const [currentStep, setCurrentStep] = createSignal(0);

  const onBoardingSteps = [
    {
      label: t("introduction"),
      icon: <div>1</div>,
      onClick: () => setCurrentStep(0),
    },
    {
      label: t("handle_java"),
      icon: <div>2</div>,
      onClick: () => setCurrentStep(1),
    },
    {
      label: t("import_instances"),
      icon: <div>3</div>,
      onClick: () => setCurrentStep(2),
    },
  ];

  const nextStep = () => {
    setCurrentStep((prev) => prev + 1);
  };

  const prevStep = () => {
    setCurrentStep((prev) => prev - 1);
  };

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      overflowHiddenDisabled={true}
    >
      <div class="select-none w-160 box-border h-145">
        <div class="max-w-70 mx-auto">
          <Steps steps={onBoardingSteps} currentStep={currentStep()} />
        </div>
        <Switch>
          <Match when={currentStep() === 0}>
            <FirstStep nextStep={nextStep} />
          </Match>
          <Match when={currentStep() === 1}>
            <SecondStep nextStep={nextStep} prevStep={prevStep} />
          </Match>
          <Match when={currentStep() === 2}>
            <ThirdStep prevStep={prevStep} />
          </Match>
        </Switch>
      </div>
    </ModalLayout>
  );
};

export default OnBoarding;
