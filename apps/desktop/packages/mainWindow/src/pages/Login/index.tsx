import { Dropdown } from "@gd/ui";
import { createSignal, Switch, Match, createEffect, Show } from "solid-js";
import Auth from "./Auth";
import CodeStep from "./CodeStep";
import fetchData from "./auth.login.data";
import { Navigate, useRouteData } from "@solidjs/router";
import { supportedLanguages, useTransContext } from "@gd/i18n";
import { queryClient, rspc } from "@/utils/rspcClient";
import TermsAndConditions from "./TermsAndConditions";
import TrackingSettings from "./TrackingSettings";
import Logo from "/assets/images/gdlauncher_vertical_logo.svg";

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
    if (routeData.settings.data?.isLegalAccepted) {
      setStep(2);
    }
  });

  const nextStep = () => {
    setStep((prev) => prev + 1);
  };

  const prevStep = () => {
    setStep((prev) => prev - 1);
  };

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
          <div class="absolute top-0 z-10 left-1/2 -translate-x-1/2 top-5">
            <Dropdown
              value={routeData.settings.data?.language}
              options={Object.keys(supportedLanguages).map((lang) => ({
                label: (
                  <div class="whitespace-nowrap">
                    {t(`languages:${lang}_native`)} {t(`languages:${lang}`)}
                  </div>
                ),
                key: lang,
              }))}
              onChange={(lang) => {
                settingsMutation.mutate({ language: lang.key as string });
              }}
              rounded
            />
          </div>
          <div
            class="flex flex-col items-center text-white relative justify-end rounded-2xl w-140 h-110"
            style={{
              background: "rgba(29, 32, 40, 0.8)",
              "justify-content": step() === 2 ? "flex-end" : "center",
            }}
            classList={{
              "overflow-hidden": step() === 3,
            }}
          >
            <Show when={step() < 2}>
              <div class="flex justify-center items-center flex-col left-0 mx-auto -mt-15">
                <img class="w-40" src={Logo} />
                <p class="text-darkSlate-50">
                  {"v"}
                  {__APP_VERSION__}
                </p>
              </div>
            </Show>
            <Show when={step() === 2}>
              <div class="absolute right-0 flex justify-center items-center flex-col left-0 m-auto -top-15">
                <img class="w-40" src={Logo} />
                <p class="text-darkSlate-50">
                  {"v"}
                  {__APP_VERSION__}
                </p>
              </div>
            </Show>
            <Switch>
              <Match when={step() === 0}>
                <TermsAndConditions nextStep={nextStep} />
              </Match>
              <Match when={step() === 1}>
                <TrackingSettings prevStep={prevStep} nextStep={nextStep} />
              </Match>
              <Match when={step() === 2}>
                <Auth
                  nextStep={nextStep}
                  setDeviceCodeObject={setDeviceCodeObject}
                />
              </Match>
              <Match when={step() === 3}>
                <CodeStep
                  nextStep={nextStep}
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
