import { Dropdown, createNotification } from "@gd/ui";
import {
  createSignal,
  Switch,
  Match,
  Show,
  createEffect,
  getOwner,
  runWithOwner
} from "solid-js";
import Auth from "./Auth";
import CodeStep from "./CodeStep";
import fetchData from "./auth.login.data";
import { Navigate, useRouteData } from "@solidjs/router";
import { Trans, supportedLanguages, useTransContext } from "@gd/i18n";
import { rspc } from "@/utils/rspcClient";
import TermsAndConditions from "./TermsAndConditions";
import Logo from "/assets/images/gdlauncher_vertical_logo.svg";
import { handleStatus } from "@/utils/login";
import { parseError } from "@/utils/helpers";

export type DeviceCodeObjectType = {
  userCode: string;
  link: string;
  expiresAt: string;
};

export default function Login() {
  const owner = getOwner();
  const [step, setStep] = createSignal<number>(0);
  const [deviceCodeObject, setDeviceCodeObject] =
    createSignal<DeviceCodeObjectType | null>(null);

  const [t, { changeLanguage }] = useTransContext();

  const routeData: ReturnType<typeof fetchData> = useRouteData();
  const isAlreadyAuthenticated = () =>
    routeData?.activeUuid?.data &&
    routeData.accounts.data?.length! > 0 &&
    routeData.settings.data?.termsAndPrivacyAccepted;

  const accountEnrollFinalizeMutation = rspc.createMutation([
    "account.enroll.finalize"
  ]);

  const nextStep = () => {
    if (step() < 2) {
      setStep((prev) => prev + 1);
    }
  };

  const prevStep = () => {
    if (step() >= 0) {
      setStep((prev) => prev - 1);
    }
  };

  const addNotification = createNotification();

  createEffect(() => {
    handleStatus(routeData.status, {
      onPolling: (info) => {
        setDeviceCodeObject({
          userCode: info.userCode,
          link: info.verificationUri,
          expiresAt: info.expiresAt
        });
        if (routeData.status.data) setStep(2);
      },
      onError(error) {
        if (error) addNotification(parseError(error), "error");
        setStep(1);
      },
      onComplete() {
        accountEnrollFinalizeMutation.mutate(undefined);
      }
    });
  });

  createEffect(() => {
    if (routeData.settings.data?.termsAndPrivacyAccepted) setStep(1);
  });

  return (
    <Switch>
      <Match when={isAlreadyAuthenticated()}>
        <Navigate href={"/library"} />
      </Match>
      <Match when={!isAlreadyAuthenticated()}>
        <div class="flex justify-center items-center w-full p-0 h-screen bg-img-loginBG.webp">
          <div
            style={{
              "mix-blend-mode": "hard-light"
            }}
            class="absolute left-0 right-0 bg-darkSlate-800 bottom-0 top-0 opacity-10"
          />
          <div class="fixed right-6 bottom-6">
            <Dropdown
              value={routeData.settings.data?.language}
              options={Object.keys(supportedLanguages).map((lang) => ({
                label: (
                  <div class="whitespace-nowrap">
                    {t(`languages:${lang}_native`)} ({t(`languages:${lang}`)})
                  </div>
                ),
                key: lang
              }))}
              onChange={(lang) => {
                runWithOwner(owner, () => {
                  changeLanguage(lang.key as string);
                });
              }}
              rounded
            />
          </div>
          <div
            class="flex items-center fixed bottom-6 gap-2 left-6 text-lightSlate-500 hover:text-lightSlate-50 transition-color duration-100 ease-in-out"
            onClick={() => {
              window.openExternalLink("https://discord.gdlauncher.com");
            }}
          >
            <div class="i-ri:lifebuoy-fill w-4 h-4" />
            <div>
              <Trans key="get_support" />
            </div>
          </div>
          <div
            class="flex flex-col items-center text-white relative justify-end rounded-2xl h-110 transition-transform duration-300 ease-in-out"
            style={{
              background: "rgba(29, 32, 40, 0.8)",
              "justify-content": step() === 1 ? "flex-end" : "center"
            }}
            classList={{
              "overflow-hidden": step() === 2,
              "w-140": step() !== 0,
              "max-w-160": step() === 0,
              "scale-100": !isAlreadyAuthenticated(),
              "scale-0": !!isAlreadyAuthenticated()
            }}
          >
            <Show when={step() === 0}>
              <div class="flex justify-center items-center flex-col left-0 mx-auto -mt-15">
                <img class="w-30" src={Logo} />
                <p class="text-darkSlate-50">
                  {"v"}
                  {__APP_VERSION__}
                </p>
              </div>
            </Show>
            <Show when={step() === 1}>
              <div class="absolute right-0 flex justify-center items-center flex-col left-0 -top-15 m-auto">
                <img class="w-30" src={Logo} />
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
                <Auth />
              </Match>
              <Match when={step() === 2}>
                <CodeStep
                  nextStep={nextStep}
                  prevStep={prevStep}
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
