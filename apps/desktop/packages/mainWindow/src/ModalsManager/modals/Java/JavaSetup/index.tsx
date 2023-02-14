import { createSignal, Match, Switch } from "solid-js";
import FirstStep from "./firstStep";
import Automatic from "./automaticStep";
import ManualStep from "./manualStep";
import ModalLayout from "@/ModalsManager/ModalLayout";
import { ModalProps } from "@/ModalsManager";

export type StepsProps = {
  nextStep?: (_step: string) => void;
};

const JavaSetup = (props: ModalProps) => {
  const [currentStep, setCurrentStep] = createSignal<string>("intro");

  const nextStep = (step: string) => {
    setCurrentStep(step);
  };

  return (
    <ModalLayout noHeader={props?.noHeader} title={props?.title}>
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
