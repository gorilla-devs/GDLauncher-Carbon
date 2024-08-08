import { Button, createNotification } from "@gd/ui";
import {
  createSignal,
  Switch,
  Match,
  createEffect,
  onMount,
  For
} from "solid-js";
import Auth from "./Auth";
import CodeStep from "./CodeStep";
import fetchData from "./auth.login.data";
import { Navigate, useRouteData } from "@solidjs/router";
import { Trans, useTransContext } from "@gd/i18n";
import { rspc } from "@/utils/rspcClient";
import TermsAndConditions from "./TermsAndConditions";
import Logo from "/assets/images/gdlauncher_wide_logo_blue.svg";
import BackgroundVideo from "/assets/images/login_background.webm";
import { handleStatus } from "@/utils/login";
import { parseError } from "@/utils/helpers";
import GDLAccount from "./GDLAccount";
import GDLAccountCompletion from "./GDLAccountCompletion";
import { useGDNavigate } from "@/managers/NavigationManager";
import GDLAccountVerification from "./GDLAccountVerification";

export type DeviceCodeObjectType = {
  userCode: string;
  link: string;
  expiresAt: string;
};

enum Steps {
  TermsAndConditions = 1,
  Auth = 2,
  CodeStep = 3,
  GDLAccount = 4,
  GDLAccountCompletion = 5,
  GDLAccountVerification = 6
}

export default function Login() {
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  const navigate = useGDNavigate();
  const [step, setStep] = createSignal<Steps>(Steps.TermsAndConditions);
  const [deviceCodeObject, setDeviceCodeObject] =
    createSignal<DeviceCodeObjectType | null>(null);
  const [loadingButton, setLoadingButton] = createSignal(false);
  const activeUuid = rspc.createQuery(() => ({
    queryKey: ["account.getActiveUuid"]
  }));

  const gdlUser = rspc.createQuery(() => ({
    queryKey: ["account.getGdlAccount", activeUuid.data!],
    enabled: !!activeUuid.data
  }));

  createEffect((prev) => {
    if (activeUuid.data && activeUuid.data !== prev) {
      gdlUser.refetch();
    }

    return activeUuid.data;
  });

  const [recoveryEmail, setRecoveryEmail] = createSignal<string | null>(null);

  const [acceptedTOS, setAcceptedTOS] = createSignal(
    routeData.settings.data?.termsAndPrivacyAccepted
  );
  const [acceptedMetrics, setAcceptedMetrics] = createSignal(
    routeData.settings.data?.metricsEnabled
  );

  const rspcContext = rspc.useContext();

  const settingsMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setSettings"]
  }));

  const deleteGDLAccountMutation = rspc.createMutation(() => ({
    mutationKey: ["account.removeGdlAccount"]
  }));

  const [isBackButtonVisible, setIsBackButtonVisible] = createSignal(false);

  const [t, { changeLanguage }] = useTransContext();

  const isAlreadyAuthenticated = () =>
    routeData?.activeUuid?.data &&
    routeData.accounts.data?.length! > 0 &&
    routeData.settings.data?.termsAndPrivacyAccepted &&
    Boolean(routeData.settings.data?.metricsEnabledLastUpdate) &&
    routeData.settings.data?.hasCompletedGdlAccountSetup;

  const accountEnrollFinalizeMutation = rspc.createMutation(() => ({
    mutationKey: ["account.enroll.finalize"]
  }));

  const nextStep = () => {
    if (step() < Steps.GDLAccountVerification) {
      setStep((prev) => prev + 1);
    }
  };

  const prevStep = () => {
    if (step() >= Steps.TermsAndConditions) {
      setStep((prev) => prev - 1);
    }
  };

  const addNotification = createNotification();

  createEffect(() => {
    handleStatus(routeData.status, {
      onPolling: async (info) => {
        setDeviceCodeObject({
          userCode: info.userCode,
          link: info.verificationUri,
          expiresAt: info.expiresAt
        });
        if (routeData.status.data) {
          await setStep(Steps.CodeStep);
          setLoadingButton(false);
        }
      },
      async onError(error) {
        if (error)
          addNotification({
            name: parseError(error),
            type: "error"
          });
        await setStep(Steps.Auth);
        setLoadingButton(false);
      },
      async onComplete() {
        await accountEnrollFinalizeMutation.mutateAsync(undefined);

        const activeUuid = await rspcContext.client.query([
          "account.getActiveUuid"
        ]);

        if (!activeUuid) {
          throw new Error("No active uuid");
        }

        const gdlUser = await rspcContext.client.query([
          "account.getGdlAccount",
          activeUuid
        ]);

        if (gdlUser?.email) {
          setRecoveryEmail(gdlUser.email);
        }

        setStep(Steps.GDLAccount);
        setLoadingButton(false);
      }
    });
  });

  onMount(async () => {
    const activeUuid = await rspcContext.client.query([
      "account.getActiveUuid"
    ]);

    const settings = await rspcContext.client.query(["settings.getSettings"]);

    if (!settings.termsAndPrivacyAccepted) {
      setStep(Steps.TermsAndConditions);
      setIsBackButtonVisible(false);
      return;
    }

    const accounts = await rspcContext.client.query(["account.getAccounts"]);

    if (!activeUuid || accounts.length === 0) {
      setStep(Steps.Auth);
      setIsBackButtonVisible(true);
      return;
    }

    const gdlUser = await rspcContext.client.query([
      "account.getGdlAccount",
      activeUuid
    ]);

    if (settings.hasCompletedGdlAccountSetup) {
      if (gdlUser && !gdlUser.isEmailVerified) {
        setRecoveryEmail(gdlUser.email);
        setStep(Steps.GDLAccountVerification);
        setIsBackButtonVisible(true);
        return;
      }

      // Should go to library if GDL account is already verified OR GDL account does not exist
      navigate("/library");
    } else if (!settings.hasCompletedGdlAccountSetup) {
      if (gdlUser) {
        setRecoveryEmail(gdlUser.email);
      }

      setIsBackButtonVisible(true);
      setStep(Steps.GDLAccount);
    }

    return;
  });

  let sidebarRef: HTMLDivElement | undefined = undefined;

  createEffect(() => {
    handleSidebarAnimation();
  });

  async function handleSidebarAnimation() {
    if (sidebarRef) {
      await new Promise((resolve) => setTimeout(resolve, 300));

      sidebarRef.animate(
        [{ transform: "translateX(-100%)" }, { transform: "translateX(0)" }],
        {
          duration: 300,
          easing: "cubic-bezier(0.175, 0.885, 0.32, 1)",
          fill: "forwards"
        }
      );
    }
  }

  let btnRef: HTMLDivElement | undefined = undefined;

  function handleAnimationForward() {
    if (btnRef) {
      if (isBackButtonVisible()) return;

      setIsBackButtonVisible(true);
      btnRef.animate(
        [
          { width: "0", margin: "0" },
          { width: "60%", margin: "0 1rem 0 0" }
        ],
        {
          duration: 300,
          easing: "cubic-bezier(0.175, 0.885, 0.32, 1.275)",
          fill: "forwards"
        }
      );
    }
  }

  function handleAnimationBackward() {
    if (btnRef && isBackButtonVisible() && step() === Steps.Auth) {
      setIsBackButtonVisible(false);

      btnRef.animate(
        [
          { width: "60%", margin: "0 1rem 0 0" },
          { width: "0", margin: "0" }
        ],
        {
          duration: 300,
          easing: "cubic-bezier(0.175, 0.885, 0.32, 1.275)",
          fill: "forwards"
        }
      );
    }
  }

  const accountEnrollCancelMutation = rspc.createMutation(() => ({
    mutationKey: ["account.enroll.cancel"]
  }));

  const accountEnrollBeginMutation = rspc.createMutation(() => ({
    mutationKey: ["account.enroll.begin"],

    onError() {
      retryLogin();
    }
  }));

  const [retry, setRetry] = createSignal(0);

  const retryLogin = () => {
    while (retry() <= 3) {
      if (!routeData.status.data) {
        accountEnrollCancelMutation.mutate(undefined);
      }
      accountEnrollBeginMutation.mutate(undefined);
      setRetry((prev) => prev + 1);
    }
    if (retry() > 3) {
      addNotification({
        name: "Something went wrong while logging in, try again!",
        type: "error"
      });
      if (routeData.status.data) {
        accountEnrollCancelMutation.mutate(undefined);
      }
    }
  };

  return (
    <Switch>
      <Match when={isAlreadyAuthenticated()}>
        <Navigate href={"/library"} />
      </Match>
      <Match when={!isAlreadyAuthenticated()}>
        <div class="flex w-full h-screen" id="main-login-page">
          <div
            ref={sidebarRef}
            class="absolute -translate-x-full w-100 h-full flex flex-col items-center text-white rounded-md bg-darkSlate-800 z-1"
          >
            <div class="flex justify-center h-30">
              <img class="w-60" src={Logo} />
            </div>
            <div class="text-lg font-bold flex items-center justify-center gap-2 mb-4">
              <Switch>
                <Match when={step() === Steps.TermsAndConditions}>
                  <Trans key="login.titles.we_value_privacy" />
                </Match>
                <Match when={step() === Steps.Auth}>
                  <Trans key="login.titles.sign_in_with_microsoft" />
                </Match>
                <Match when={step() === Steps.CodeStep}>
                  <i class="inline-block w-4 h-4 i-ri:microsoft-fill" />
                  <Trans key="login.titles.microsoft_code_step" />
                </Match>
                <Match when={step() === Steps.GDLAccount}>
                  <Switch>
                    <Match when={gdlUser.data}>
                      <Trans key="login.titles.sync_gdl_account" />
                    </Match>
                    <Match when={!gdlUser.data}>
                      <Trans key="login.titles.create_gdl_account" />
                    </Match>
                  </Switch>
                </Match>
                <Match when={step() === Steps.GDLAccountCompletion}>
                  <Trans key="login.titles.linked_microsoft_account" />
                </Match>
                <Match when={step() === Steps.GDLAccountVerification}>
                  <Trans key="login.titles.gdl_account_verification" />
                </Match>
              </Switch>
            </div>
            <div class="flex flex-1 w-full h-auto overflow-y-auto px-4 box-border">
              <Switch>
                <Match when={step() === Steps.TermsAndConditions}>
                  <TermsAndConditions
                    nextStep={nextStep}
                    acceptedTOS={!!acceptedTOS()}
                    setAcceptedTOS={setAcceptedTOS}
                    acceptedMetrics={!!acceptedMetrics()}
                    setAcceptedMetrics={setAcceptedMetrics}
                  />
                </Match>
                <Match when={step() === Steps.Auth}>
                  <Auth />
                </Match>
                <Match when={step() === Steps.CodeStep}>
                  <CodeStep
                    nextStep={nextStep}
                    prevStep={prevStep}
                    deviceCodeObject={deviceCodeObject()}
                    setDeviceCodeObject={setDeviceCodeObject}
                  />
                </Match>
                <Match when={step() === Steps.GDLAccount}>
                  <GDLAccount activeUuid={activeUuid.data} />
                </Match>
                <Match when={step() === Steps.GDLAccountCompletion}>
                  <GDLAccountCompletion
                    nextStep={nextStep}
                    prevStep={prevStep}
                    recoveryEmail={recoveryEmail()}
                    setRecoveryEmail={setRecoveryEmail}
                  />
                </Match>
                <Match when={step() === Steps.GDLAccountVerification}>
                  <GDLAccountVerification
                    nextStep={nextStep}
                    prevStep={prevStep}
                    activeUuid={activeUuid.data}
                  />
                </Match>
              </Switch>
            </div>

            <div class="w-full flex flex-col items-center p-4 box-border">
              <div class="relative flex justify-center gap-2 mb-4">
                <div class="absolute top-1/2 left-0 -translate-y-1/2 h-4 w-full rounded-lg overflow-hidden">
                  <div
                    class="absolute top-0 left-0 bg-darkSlate-400 h-4 w-full rounded-lg"
                    style={{
                      transform: `translateX(calc((-100% + ${(100 * step()) / 6}%) - ${(step() === Steps.TermsAndConditions ? 8 : 6) - step()}px)`,
                      transition:
                        "transform 0.3s cubic-bezier(0.175, 0.885, 0.32, 1.275)"
                    }}
                  />
                </div>

                <For each={new Array(6)}>
                  {(_, i) => (
                    <div
                      class="z-1 h-6 w-4 flex justify-center items-center group"
                      onClick={() => {
                        if (
                          i() + 1 < step() &&
                          (step() > Steps.CodeStep
                            ? i() + 1 > Steps.CodeStep
                            : true)
                        )
                          setStep(i() + 1);
                      }}
                    >
                      <div
                        class="h-2 w-2 bg-lightSlate-900 rounded-full"
                        classList={{
                          "group-hover:bg-lightSlate-100":
                            i() + 1 < step() &&
                            (step() > Steps.CodeStep
                              ? i() + 1 > Steps.CodeStep
                              : true)
                        }}
                      />
                    </div>
                  )}
                </For>
              </div>

              <div class="flex w-full box-border">
                <div
                  ref={btnRef}
                  class="overflow-hidden"
                  style={{
                    width: !isBackButtonVisible() ? "0" : "60%",
                    margin: !isBackButtonVisible() ? "0" : "0 1rem 0 0"
                  }}
                >
                  <Button
                    size="large"
                    type="secondary"
                    fullWidth
                    onClick={async () => {
                      if (step() === Steps.GDLAccount) {
                        await deleteGDLAccountMutation.mutateAsync(undefined);
                        navigate("/library");
                      } else {
                        handleAnimationBackward();
                        prevStep();
                      }

                      setLoadingButton(false);
                    }}
                  >
                    <Switch>
                      <Match when={step() !== Steps.GDLAccount}>
                        <i class="i-ri:arrow-left-line" />
                        Back
                      </Match>
                      <Match when={step() === Steps.GDLAccount}>
                        Skip
                        <i class="i-ri:skip-forward-line" />
                      </Match>
                    </Switch>
                  </Button>
                </div>
                <Button
                  fullWidth
                  variant="primary"
                  size="large"
                  disabled={
                    !acceptedTOS() ||
                    step() === Steps.CodeStep ||
                    step() === Steps.GDLAccountVerification ||
                    (step() === Steps.GDLAccountCompletion && !recoveryEmail())
                  }
                  loading={
                    loadingButton() || step() === Steps.GDLAccountVerification
                  }
                  onClick={async () => {
                    handleAnimationForward();
                    setLoadingButton(true);

                    if (step() === Steps.TermsAndConditions) {
                      try {
                        await settingsMutation.mutateAsync({
                          termsAndPrivacyAccepted: {
                            Set: true
                          },
                          metricsEnabled: {
                            Set: !!acceptedMetrics()
                          }
                        });
                      } catch (err) {
                        console.log(err);
                        addNotification({
                          name: "Error while accepting terms and conditions",
                          content: "Check the console for more information.",
                          type: "error"
                        });
                      }

                      setLoadingButton(false);
                      nextStep();
                    } else if (step() === Steps.Auth) {
                      if (!routeData.status.data) {
                        await accountEnrollBeginMutation.mutateAsync(undefined);
                      } else {
                        await accountEnrollCancelMutation.mutateAsync(
                          undefined
                        );
                        await accountEnrollBeginMutation.mutateAsync(undefined);
                      }
                    } else if (step() === Steps.GDLAccount) {
                      const uuid = routeData?.activeUuid?.data;

                      if (!uuid) {
                        throw new Error("No active uuid");
                      }

                      try {
                        const existingGDLUser = await rspcContext.client.query([
                          "account.getGdlAccount",
                          uuid
                        ]);

                        console.log(existingGDLUser);

                        if (
                          existingGDLUser &&
                          existingGDLUser.isEmailVerified
                        ) {
                          navigate("/library");
                          return;
                        } else if (
                          existingGDLUser &&
                          !existingGDLUser.isEmailVerified
                        ) {
                          setRecoveryEmail(existingGDLUser.email);
                          setStep(Steps.GDLAccountVerification);
                          return;
                        }
                      } catch (e) {
                        console.error(e);
                      }

                      await settingsMutation.mutateAsync({
                        hasCompletedGdlAccountSetup: {
                          Set: false
                        }
                      });
                      setLoadingButton(false);
                      nextStep();
                    } else if (step() === Steps.GDLAccountCompletion) {
                      const uuid = routeData?.activeUuid?.data;

                      if (!uuid) {
                        throw new Error("No active uuid");
                      }

                      const email = recoveryEmail();

                      if (!email) {
                        throw new Error("No recovery email");
                      }

                      try {
                        const existingGDLUser = await rspcContext.client.query([
                          "account.getGdlAccount",
                          uuid
                        ]);

                        if (
                          existingGDLUser &&
                          existingGDLUser.isEmailVerified
                        ) {
                          navigate("/library");
                        } else if (
                          existingGDLUser &&
                          !existingGDLUser.isEmailVerified
                        ) {
                          setRecoveryEmail(existingGDLUser.email);
                          setStep(Steps.GDLAccountVerification);
                        } else {
                          const gdlUser = await rspcContext.client.mutation([
                            "account.registerGdlAccount",
                            {
                              email,
                              uuid
                            }
                          ]);

                          console.log("GDL USER", gdlUser);
                          nextStep();
                        }
                      } catch (e) {
                        setLoadingButton(false);
                        console.error(e);
                      }
                    }
                  }}
                >
                  <Switch>
                    <Match
                      when={
                        step() === Steps.GDLAccountCompletion && !gdlUser.data
                      }
                    >
                      <Trans key="login.register" />
                      <i class="i-ri:arrow-right-line" />
                    </Match>
                    <Match
                      when={
                        step() === Steps.GDLAccount &&
                        gdlUser.data &&
                        gdlUser.data.isEmailVerified
                      }
                    >
                      <Trans key="login.sync_gdl_account" />
                      <i class="i-ri:arrow-right-line" />
                    </Match>
                    <Match when={step() === Steps.Auth}>
                      <i class="w-4 h-4 i-ri:microsoft-fill" />
                      <Trans key="login.sign_in" />
                    </Match>
                    <Match when={step() !== Steps.Auth}>
                      <Trans key="login.next" />
                      <i class="i-ri:arrow-right-line" />
                    </Match>
                  </Switch>
                </Button>
              </div>
            </div>
          </div>
          <div class="flex-1">
            <video
              class="p-0 h-screen w-full object-cover"
              src={BackgroundVideo}
              autoplay
              muted
              loop
              playsinline
            />
            {/* <div
              style={{
                "mix-blend-mode": "hard-light"
              }}
              class="absolute left-0 right-0 bg-darkSlate-800 bottom-0 top-0 opacity-30"
            /> */}
          </div>
        </div>
      </Match>
    </Switch>
  );
}
