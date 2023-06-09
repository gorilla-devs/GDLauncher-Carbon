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

  const onBoardingSteps = [
    { label: t("introduction"), icon: <div>1</div> },
    { label: t("handle_java"), icon: <div>2</div> },
    { label: t("import_instances"), icon: <div>3</div> },
  ];
  const [currentStep, setCurrentStep] = createSignal(0);

  const nextStep = () => {
    setCurrentStep((prev) => prev + 1);
  };

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      overflowHiddenDisabled={true}
    >
      <div class="select-none">
        <div
          class="max-w-70 mx-auto"
          classList={{
            hidden: currentStep() === 2,
          }}
        >
          <Steps steps={onBoardingSteps} currentStep={currentStep()} />
        </div>
        <Switch>
          <Match when={currentStep() === 0}>
            <FirstStep nextStep={nextStep} />
          </Match>
          <Match when={currentStep() === 1}>
            <SecondStep nextStep={nextStep} />
          </Match>
          <Match when={currentStep() === 2}>
            <ThirdStep />
          </Match>
        </Switch>
      </div>
    </ModalLayout>
  );
};

export default OnBoarding;
