import { For, Show, JSX, mergeProps, Switch, Match } from "solid-js";

type CustomStep = {
  icon?: string | JSX.Element;
  label: string;
};

type Props = {
  steps: string[] | CustomStep[];
  currentStep?: number;
  class?: string;
};

const Steps = (props: Props) => {
  const mergedProps = mergeProps({ currentStep: 0 }, props);

  return (
    <div class={`flex flex-col gap-3 w-full ${props.class || ""}`}>
      <div class="flex items-center w-full">
        <For each={props.steps}>
          {(step, i) => (
            <>
              <div
                class="relative bg-primary w-6 h-6 rounded-full flex justify-center items-center"
                classList={{
                  "text-sm": typeof step === "string",
                  "bg-primary": i() <= mergedProps.currentStep,
                  "bg-shade-5": i() > mergedProps.currentStep,
                }}
              >
                <Switch>
                  <Match when={typeof step === "string"}>
                    {step as string}
                  </Match>
                  <Match when={typeof step === "object"}>
                    {(step as CustomStep)?.icon || i() + 1}
                  </Match>
                </Switch>
                <Show when={typeof step === "object"}>
                  <div class="absolute -bottom-8 whitespace-nowrap	">
                    {(step as CustomStep).label}
                  </div>
                </Show>
              </div>

              <Show when={i() !== props.steps.length - 1}>
                <div
                  class="flex-auto border-t-2 border-y-0 border-b-0"
                  classList={{
                    "border-solid border-primary":
                      i() < mergedProps.currentStep,
                    "border-dashed border-shade-5":
                      i() >= mergedProps.currentStep,
                  }}
                />
              </Show>
            </>
          )}
        </For>
      </div>
    </div>
  );
};

export { Steps };
