import { createSignal, Switch, Match } from "solid-js";
import Auth from "./Auth";
import CodeStep from "./CodeStep";

export default function Login() {
  const [step, setStep] = createSignal(0);
  const [deviceCodeObject, setDeviceCodeObject] = createSignal<any>(null);

  return (
    <div class="flex justify-center items-center w-full h-full bg-image-loginBG.jpg p-0">
      <div
        style={{
          "mix-blend-mode": "hard-light",
        }}
        class="absolute top-0 left-0 right-0 bottom-0 bg-black-black opacity-80"
      />
      <div
        class="w-120 h-100 rounded-2xl relative flex flex-col justify-end items-center text-white"
        style={{
          background: "rgba(29, 32, 40, 0.8)",
          "justify-content": step() === 0 ? "flex-end" : "center",
        }}
      >
        <Switch
          fallback={
            <Auth setStep={setStep} setDeviceCodeObject={setDeviceCodeObject} />
          }
        >
          <Match when={step() === 0}>
            <Auth setStep={setStep} setDeviceCodeObject={setDeviceCodeObject} />
          </Match>
          <Match when={step() === 1}>
            <CodeStep
              deviceCodeObject={deviceCodeObject()}
              setDeviceCodeObject={setDeviceCodeObject}
            />
          </Match>
        </Switch>
      </div>
    </div>
  );
}
