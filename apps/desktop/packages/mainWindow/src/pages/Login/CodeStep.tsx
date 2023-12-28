import { Button, LoadingBar, Popover } from "@gd/ui";
import { createEffect, createSignal, onCleanup, Show } from "solid-js";
import { msToMinutes, msToSeconds, parseTwoDigitNumber } from "@/utils/helpers";
import { Setter } from "solid-js";
import { DeviceCode } from "@/components/CodeInput";
import { createNotification } from "@gd/ui";
import { Trans, useTransContext } from "@gd/i18n";
import { rspc } from "@/utils/rspcClient";
import { DeviceCodeObjectType } from ".";
import GateAnimationRiveWrapper from "@/utils/GateAnimationRiveWrapper";
import GateAnimation from "../../gate_animation.riv";
import { handleStatus } from "@/utils/login";
import { useRouteData } from "@solidjs/router";
import fetchData from "./auth.login.data";
import { EnrollmentError } from "@gd/core_module/bindings";

interface Props {
  deviceCodeObject: DeviceCodeObjectType | null;
  setDeviceCodeObject: Setter<DeviceCodeObjectType | null>;
  nextStep: () => void;
  prevStep: () => void;
}

const CodeStep = (props: Props) => {
  const [error, setError] = createSignal<null | string>(null);

  const accountEnrollCancelMutation = rspc.createMutation(
    ["account.enroll.cancel"],
    {
      onError(error) {
        setError(error.message);
      }
    }
  );

  const accountEnrollBeginMutation = rspc.createMutation(
    ["account.enroll.begin"],
    {
      onError(error) {
        setError(error.message);
      }
    }
  );

  const userCode = () => props.deviceCodeObject?.userCode;
  const oldUserCode = () => props.deviceCodeObject?.userCode;
  const deviceCodeLink = () => props.deviceCodeObject?.link;
  const expiresAt = () => props.deviceCodeObject?.expiresAt;
  const expiresAtFormat = () => new Date(expiresAt() || "")?.getTime();
  const expiresAtMs = () => expiresAtFormat() - Date.now();
  const minutes = () => msToMinutes(expiresAtMs());
  const seconds = () => msToSeconds(expiresAtMs());
  const [countDown, setCountDown] = createSignal(
    `${minutes()}:${parseTwoDigitNumber(seconds())}`
  );
  const [expired, setExpired] = createSignal(false);
  const [t] = useTransContext();

  const resetCountDown = () => {
    setExpired(false);
    if (minutes() >= 0 && seconds() > 0) {
      setCountDown(`${minutes()}:${parseTwoDigitNumber(seconds())}`);
    }
  };
  const [loading, setLoading] = createSignal(false);

  const handleRefersh = async () => {
    resetCountDown();
    if (routeData.status.data) {
      accountEnrollCancelMutation.mutate(undefined);
    }
    accountEnrollBeginMutation.mutate(undefined);
  };

  const updateExpireTime = () => {
    if (minutes() <= 0 && seconds() <= 0) {
      setLoading(false);
      setExpired(true);
    } else {
      resetCountDown();
    }
  };

  let interval: ReturnType<typeof setTimeout>;
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  createEffect(() => {
    if (expired()) {
      if (routeData.status.data) accountEnrollCancelMutation.mutate(undefined);
      setLoading(false);
      clearInterval(interval);
      setCountDown(`${minutes()}:${parseTwoDigitNumber(seconds())}`);
    } else {
      interval = setInterval(() => {
        updateExpireTime();
      }, 1000);
    }
  });

  createEffect(() => {
    if (userCode() !== oldUserCode()) {
      resetCountDown();
    }
  });

  const handleErrorMessages = (error: EnrollmentError) => {
    const isCodeExpired = error === "deviceCodeExpired";

    if (isCodeExpired) {
      handleRefersh();
    } else if (typeof error === "string") {
      addNotification(t(`error.${error}`), "error");
    } else {
      if (typeof error.xboxAccount === "string")
        addNotification(t(`error.xbox_${error.xboxAccount}`), "error");
      else
        addNotification(
          `${t("error.xbox_code")} ${error.xboxAccount.unknown}`,
          "error"
        );
    }
  };

  createEffect(() => {
    handleStatus(routeData.status, {
      onFail(error) {
        handleErrorMessages(error);
      }
    });
  });

  onCleanup(() => clearInterval(interval));

  const addNotification = createNotification();

  return (
    <div class="flex flex-col justify-between items-center text-center gap-5 p-10">
      <GateAnimationRiveWrapper width={80} height={80} src={GateAnimation} />
      <div class="absolute top-4 left-4">
        <Button
          type="secondary"
          onClick={() => {
            props.prevStep();
          }}
        >
          <Trans
            key="login.step_back"
            options={{
              defaultValue: "Back"
            }}
          />
        </Button>
      </div>
      <div class="absolute top-4 right-4">
        <Popover
          noTip
          content={
            <div class="px-4 pb-6 max-w-100">
              <h3>
                <Trans key="login.troubles_logging_in" />
              </h3>
              <div class="text-md pb-8">
                <Trans key="login.link_not_working_help" />
              </div>
              <div
                class="text-lightSlate-600 hover:text-lightSlate-50 flex gap-2 items-center"
                onClick={() => {
                  navigator.clipboard.writeText(deviceCodeLink()!);
                  addNotification("The link has been copied");
                }}
              >
                <div class="w-4 h-4 i-ri:link" />
                <div>{deviceCodeLink()}</div>
              </div>
            </div>
          }
        >
          <div class="flex items-center text-darkSlate-50 hover:text-lightSlate-50 transition-color duration-75">
            <div>
              <Trans key="login.need_help" />
            </div>
            <div class="ml-2 w-6 h-6 i-ri:question-fill" />
          </div>
        </Popover>
      </div>
      <div>
        <div class="flex flex-col justify-center items-center">
          <DeviceCode
            disabled={expired()}
            value={userCode() || ""}
            id="login-link-btn"
            onClick={() => {
              window.copyToClipboard(userCode() || "");
              addNotification("The code has been copied");
            }}
          />
          <Show when={expired()}>
            <p class="mb-0 mt-2 text-red-500">
              <Trans
                key="login.code_expired_message"
                options={{
                  defaultValue: "The code has been expired"
                }}
              />
            </p>
          </Show>
        </div>
        <Show when={!expired()}>
          <p class="mb-0 text-darkSlate-50 mt-4">
            <span class="text-white mr-2">{countDown()}</span>
            <Trans
              key="login.before_expiring"
              options={{
                defaultValue: "before the code expires"
              }}
            />
          </p>
          <p class="text-darkSlate-50 mb-0">
            <Trans
              key="login.enter_code_in_browser"
              options={{
                defaultValue:
                  "Enter the specified code on the browser page to complete the authorization"
              }}
            />
          </p>
        </Show>
      </div>
      <Show when={error()}>
        <p class="text-red-500 m-0">{error()}</p>
      </Show>
      <Show when={loading()}>
        <span class="mb-4 text-xs absolute text-darkSlate-100 bottom-1">
          <Trans
            key="login.waiting_login_code_msg"
            options={{
              defaultValue: "Waiting for authorization..."
            }}
          />
        </span>
        <div class="w-full absolute overflow-hidden bottom-0">
          <LoadingBar class="" />
        </div>
      </Show>
      <Show when={!expired()}>
        <Button
          id="login-btn"
          class="normal-case"
          onClick={() => {
            setLoading(true);
            navigator.clipboard.writeText(userCode() || "");
            window.openExternalLink(deviceCodeLink() || "");
          }}
        >
          <Trans
            key="login.open_in_browser"
            options={{
              defaultValue: "Copy and open in browser"
            }}
          />
          <div class="text-md i-ri:external-link-fill" />
        </Button>
      </Show>
      <Show when={expired()}>
        <div
          class="flex justify-between items-center gap-2 cursor-pointer"
          onClick={() => handleRefersh()}
        >
          <span class="i-ri:refresh-line" />
          <h3 class="m-0">
            <Trans
              key="login.refresh"
              options={{
                defaultValue: "refresh"
              }}
            />
          </h3>
        </div>
      </Show>
    </div>
  );
};

export default CodeStep;
