import { createSignal, Match, Switch } from "solid-js";
import FirstStep from "./firstStep";
import Automatic from "./automaticStep";
import ManualStep from "./manualStep";
import ModalLayout from "@/managers/ModalsManager/ModalLayout";
import { ModalProps } from "@/managers/ModalsManager";

export type StepsProps = {
  nextStep?: (_step: string) => void;
};

const JavaSetup = (props: ModalProps) => {
  const [currentStep, setCurrentStep] = createSignal<string>("intro");

  const nextStep = (step: string) => {
    setCurrentStep(step);
  };

  return (
    <ModalLayout
      noHeader={currentStep() === "automatic"}
      title={props?.title}
      preventClose={currentStep() === "automatic"}
    >
      <div>
        <Switch>
          <Match when={currentStep() === "intro"}>
            <FirstStep nextStep={nextStep} />
          </Match>
          <Match when={currentStep() === "automatic"}>
            <Automatic />
          </Match>
          <Match when={currentStep() === "manual"}>
            <ManualStep nextStep={nextStep} />
          </Match>
        </Switch>
      </div>
    </ModalLayout>
  );
};

export default JavaSetup;
