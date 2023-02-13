import { createSignal, Match, Switch } from "solid-js";
import FirstStep from "./firstStep";
import Automatic from "./automaticStep";
import ManualStep from "./manualStep";

export type StepsProps = {
  nextStep?: (_step: string) => void;
};

const JavaSetup = () => {
  const [currentStep, setCurrentStep] = createSignal<string>("intro");

  const nextStep = (step: string) => {
    setCurrentStep(step);
  };

  return (
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
  );
};

export default JavaSetup;
