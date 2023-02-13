import { createEffect, createSignal, JSX, Match, Switch } from "solid-js";
import FirstStep from "./firstStep";
import SecondStep from "./secondStep";

export type StepsProps = {
  nextStep?: () => void;
  previusStep?: () => void;
};

type Step = (_props: StepsProps) => JSX.Element;

type Steps = { [key: number]: Step };

const JavaSetup = () => {
  const [currentStep, setCurrentStep] = createSignal<number>(0);

  const nextStep = () => {
    setCurrentStep((prev) => {
      console.log("AAA", prev, prev < 1);
      if (prev < 1) return prev + 1;
      else return prev;
    });
  };

  const previusStep = () => {
    setCurrentStep((prev) => {
      if (prev > 0) return prev - 1;
      else return prev;
    });
  };

  createEffect(() => {
    console.log("TEST", currentStep());
  });

  return (
    <>
      <Switch>
        <Match when={currentStep() === 0}>
          <FirstStep nextStep={nextStep} previusStep={previusStep} />
        </Match>
        <Match when={currentStep() === 1}>
          <SecondStep nextStep={nextStep} previusStep={previusStep} />
        </Match>
      </Switch>
    </>
  );
};

export default JavaSetup;
