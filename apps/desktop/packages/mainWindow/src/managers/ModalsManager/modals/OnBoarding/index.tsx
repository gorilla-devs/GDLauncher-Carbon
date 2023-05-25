import ModalLayout from "../../ModalLayout";
import { ModalProps } from "../..";
import { Steps } from "@gd/ui";
import { Match, Switch, createSignal } from "solid-js";
import SecondStep from "./SecondStep";
import ThirdStep from "./ThirdStep";
import FirstStep from "./FirstStep";

export const onBoardingSteps = [
  { label: "Introduction", icon: <div>1</div> },
  { label: "handle java", icon: <div>2</div> },
  { label: "Import instances", icon: <div>3</div> },
];

const OnBoarding = (props: ModalProps) => {
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
          class="mx-auto max-w-60"
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
