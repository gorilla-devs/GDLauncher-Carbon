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
    <div class="w-110 h-80">
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
