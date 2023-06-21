import { Dropdown } from "@gd/ui";
import { createSignal, Switch, Match, createEffect } from "solid-js";
import Auth from "./Auth";
import CodeStep from "./CodeStep";
import fetchData from "./auth.login.data";
import { Navigate, useRouteData } from "@solidjs/router";
import { useTransContext } from "@gd/i18n";
import { queryClient, rspc } from "@/utils/rspcClient";
import TermsAndConditions from "./TermsAndConditions";

export type DeviceCodeObjectType = {
  userCode: string;
  link: string;
  expiresAt: string;
};

export default function Login() {
  const [step, setStep] = createSignal<number>(0);
  const [deviceCodeObject, setDeviceCodeObject] =
    createSignal<DeviceCodeObjectType | null>(null);

  const [t, { changeLanguage }] = useTransContext();

  const routeData: ReturnType<typeof fetchData> = useRouteData();
  const isAlreadyAuthenticated = () =>
    routeData?.activeUuid?.data && routeData?.accounts?.data?.length! > 0;

  const settingsMutation = rspc.createMutation(["settings.setSettings"], {
    onMutate: (newSettings) => {
      if (newSettings.language) changeLanguage(newSettings.language as string);
      queryClient.setQueryData(["settings.getSettings"], newSettings);
    },
  });

  createEffect(() => {
    // if already accepted go to the next step
  });

  return (
    <Switch>
      <Match when={isAlreadyAuthenticated()}>
        <Navigate href={"/library"} />
      </Match>
      <Match when={!isAlreadyAuthenticated()}>
        <div class="flex justify-center items-center w-full h-screen p-0 bg-img-loginBG.jpg">
          <div
            style={{
              "mix-blend-mode": "hard-light",
            }}
            class="absolute left-0 right-0 bg-darkSlate-800 top-0 bottom-0 opacity-80"
          />
          <div class="absolute top-0 top-5 z-10 left-1/2 -translate-x-1/2">
            <Dropdown
              value={routeData.settings.data?.language || "en"}
              options={[
                { label: t("languages.english"), key: "eng" },
                { label: t("languages.italian"), key: "it" },
              ]}
              onChange={(lang) => {
                settingsMutation.mutate({ language: lang.key as string });
              }}
              rounded
            />
          </div>
          <div
            class="flex flex-col items-center text-white relative justify-end rounded-2xl w-120 h-100"
            style={{
              background: "rgba(29, 32, 40, 0.8)",
              "justify-content": step() === 1 ? "flex-end" : "center",
            }}
            classList={{
              "overflow-hidden": step() === 2,
            }}
          >
            <Switch>
              <Match when={step() === 0}>
                <TermsAndConditions setStep={setStep} />
              </Match>
              <Match when={step() === 1}>
                <Auth
                  setStep={setStep}
                  setDeviceCodeObject={setDeviceCodeObject}
                />
              </Match>
              <Match when={step() === 2}>
                <CodeStep
                  setStep={setStep}
                  deviceCodeObject={deviceCodeObject()}
                  setDeviceCodeObject={setDeviceCodeObject}
                />
              </Match>
            </Switch>
          </div>
        </div>
      </Match>
    </Switch>
  );
}
