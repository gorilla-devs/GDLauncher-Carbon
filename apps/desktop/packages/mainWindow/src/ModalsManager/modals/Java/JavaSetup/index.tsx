import { createSignal, Match, Switch } from "solid-js";
import FirstStep from "./firstStep";
import SecondStep from "./secondStep";

export type StepsProps = {
  nextStep?: () => void;
  previusStep?: () => void;
};

const JavaSetup = () => {
  const [currentStep, setCurrentStep] = createSignal<number>(0);

  const nextStep = () => {
    console.log("NEXT", currentStep(), currentStep() < 1);
    if (currentStep() < 1) {
      setCurrentStep((prev) => prev + 1);
    }
  };

  const previusStep = () => {
    if (currentStep() > 0) {
      setCurrentStep((prev) => prev - 1);
    }
  };

  return (
    <div>
      <Switch>
        <Match when={currentStep() === 0}>
          <FirstStep nextStep={nextStep} previusStep={previusStep} />
        </Match>
        <Match when={currentStep() === 1}>
          <SecondStep nextStep={nextStep} previusStep={previusStep} />
        </Match>
      </Switch>
    </div>
  );
};

export default JavaSetup;
