import { Button, LoadingBar } from "@gd/ui";
import { useRouteData } from "@solidjs/router";
import DoorImage from "/assets/images/door.png";
import { createEffect, createSignal, onCleanup, Show } from "solid-js";
import { msToMinutes, msToSeconds, parseTwoDigitNumber } from "@/utils/helpers";
import { Setter } from "solid-js";
import { DeviceCode } from "@/components/CodeInput";
import { createNotification } from "@gd/ui";
import { Trans } from "@gd/i18n";
import { queryClient, rspc } from "@/utils/rspcClient";
import fetchData from "./auth.login.data";
import { handleStatus } from "@/utils/login";
import { useGDNavigate } from "@/managers/NavigationManager";
import { DeviceCodeObjectType } from ".";
import { Procedures } from "@gd/core_module";
interface Props {
  deviceCodeObject: DeviceCodeObjectType | null;
  setDeviceCodeObject: Setter<DeviceCodeObjectType>;
  setStep: Setter<number>;
}

type ActiveUUID = Extract<
  Procedures["queries"],
  { key: "account.setActiveUuid" }
>["result"];

const CodeStep = (props: Props) => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();
  const navigate = useGDNavigate();
  const [enrollmentInProgress, setEnrollmentInProgress] = createSignal(false);
  const [error, setError] = createSignal<null | string>(null);

  const accountEnrollCancelMutation = rspc.createMutation(
    ["account.enroll.cancel"],
    {
      onError(error) {
        setError(error.message);
      },
    }
  );

  const accountEnrollBeginMutation = rspc.createMutation(
    ["account.enroll.begin"],
    {
      onError(error) {
        setError(error.message);
      },
    }
  );

  const finalizeMutation = rspc.createMutation(["account.enroll.finalize"]);

  const setActiveUUIDMutation = rspc.createMutation(["account.setActiveUuid"], {
    onMutate: async (
      uuid
    ): Promise<{ previousActiveUUID: ActiveUUID } | undefined> => {
      await queryClient.cancelQueries({ queryKey: ["account.setActiveUuid"] });

      const previousActiveUUID: ActiveUUID | undefined =
        queryClient.getQueryData(["account.setActiveUuid"]);

      queryClient.setQueryData(["account.setActiveUuid", null], uuid);

      if (previousActiveUUID) return { previousActiveUUID };
    },
    onError: (
      error,
      _variables,
      context: { previousActiveUUID: ActiveUUID } | undefined
    ) => {
      addNotification(error.message, "error");

      if (context?.previousActiveUUID) {
        queryClient.setQueryData(
          ["account.setActiveUuid"],
          context.previousActiveUUID
        );
      }
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: ["account.setActiveUuid"] });
    },
    onSuccess() {
      navigate("/library");
      setEnrollmentInProgress(false);
    },
  });

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

  const resetCountDown = () => {
    setExpired(false);
    setCountDown(`${minutes()}:${parseTwoDigitNumber(seconds())}`);
  };
  const [loading, setLoading] = createSignal(false);

  const handleRefersh = async () => {
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

  createEffect(() => {
    if (routeData.status.isSuccess && !routeData.status.data) {
      setEnrollmentInProgress(false);
    } else {
      setEnrollmentInProgress(true);
    }

    handleStatus(routeData.status, {
      onPolling: (info) => {
        setEnrollmentInProgress(true);
        props.setDeviceCodeObject({
          userCode: info.userCode,
          link: info.verificationUri,
          expiresAt: info.expiresAt,
        });
        setExpired(false);
        setError(null);
      },
      onFail() {
        setEnrollmentInProgress(false);
        setError("something went wrong while logging in");
        props.setStep(0);
      },
      onComplete(account) {
        finalizeMutation.mutate(undefined);
        // if (finalizeMutation.isSuccess) {
        setActiveUUIDMutation.mutate(account.uuid);
        // }
      },
    });
  });

  createEffect(() => {
    if (expired()) {
      if (enrollmentInProgress()) accountEnrollCancelMutation.mutate(undefined);
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

  onCleanup(() => clearInterval(interval));

  const addNotification = createNotification();

  return (
    <div class="flex flex-col justify-between items-center text-center gap-5 p-10">
      <img src={DoorImage} />
      <div>
        <div class="flex flex-col justify-center items-center">
          <DeviceCode
            disabled={expired()}
            value={userCode() || ""}
            onClick={() => {
              navigator.clipboard.writeText(userCode() || "");
              addNotification("The link has been copied");
            }}
          />
          <Show when={expired()}>
            <p class="mb-0 mt-2 text-[#E54B4B]">
              <Trans
                key="login.code_expired_message"
                options={{
                  defaultValue: "The code has been expired",
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
                defaultValue: "before the code expires",
              }}
            />
          </p>
          <p class="text-darkSlate-50 mb-0">
            <Trans
              key="login.enter_code_in_browser"
              options={{
                defaultValue:
                  "Enter the specified code on the browser page to complete the authorization",
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
              defaultValue: "Waiting for authorization...",
            }}
          />
        </span>
        <div class="w-full absolute overflow-hidden bottom-0">
          <LoadingBar class="" />
        </div>
      </Show>
      <Show when={!expired()}>
        <Button
          class="normal-case"
          onClick={() => {
            setLoading(true);
            window.openExternalLink(deviceCodeLink() || "");
          }}
        >
          <Trans
            key="login.open_in_browser"
            options={{
              defaultValue: "Open in browser",
            }}
          />
          <div class="text-md i-ri:link" />
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
                defaultValue: "refresh",
              }}
            />
          </h3>
        </div>
      </Show>
    </div>
  );
};

export default CodeStep;
