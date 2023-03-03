import { Dropdown } from "@gd/ui";
import { createSignal, Switch, Match } from "solid-js";
import Auth from "./Auth";
import CodeStep from "./CodeStep";

export default function Login() {
  const [step, setStep] = createSignal(0);
  const [deviceCodeObject, setDeviceCodeObject] = createSignal<any>(null);

  return (
    <div class="flex justify-center items-center w-full h-screen p-0 bg-img-loginBG.jpg">
      <div
        style={{
          "mix-blend-mode": "hard-light",
        }}
        class="absolute left-0 right-0 bg-shade-8 top-0 bottom-0 opacity-80"
      />
      <div class="absolute top-0 top-5 z-10 left-1/2 -translate-x-1/2">
        <Dropdown
          options={[
            { label: "english", key: "en" },
            { label: "italian", key: "it" },
          ]}
          value={"asc"}
          onChange={() => {
            // getTranslationByLanguage(lang.key).then((translations) => {
            //   add(lang.key, translations);
            //   locale(lang.key);
            // });
          }}
          rounded
        />
      </div>
      <div
        class="flex flex-col items-center text-white relative justify-end rounded-2xl w-120 h-100"
        style={{
          background: "rgba(29, 32, 40, 0.8)",
          "justify-content": step() === 0 ? "flex-end" : "center",
        }}
        classList={{
          "overflow-hidden": step() === 1,
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
