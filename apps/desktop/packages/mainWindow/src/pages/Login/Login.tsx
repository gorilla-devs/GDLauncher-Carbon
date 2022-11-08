import { createSignal, Switch, Match, onMount } from "solid-js";
import Auth from "./Auth";
import CodeStep from "./CodeStep";

export const [step, setStep] = createSignal(0);

export default function Login() {
  onMount(() => {
    setStep(0);
  });

  return (
    <div class="flex justify-center items-center w-full h-full bg-image-loginBG p-0">
      <div
        style={{
          "mix-blend-mode": "hard-light",
        }}
        class="absolute top-0 left-0 right-0 bottom-0 bg-[#1D2028] opacity-80"
      />
      <div class="w-120 h-90 rounded-2xl bg-[#1D2028] opacity-80 relative backdrop-blur-sm flex flex-col justify-end items-center text-white">
        <Switch fallback={<Auth />}>
          <Match when={step() === 0}>
            <Auth />
          </Match>
          <Match when={step() === 1}>
            <CodeStep />
          </Match>
        </Switch>
      </div>
    </div>
  );
}
