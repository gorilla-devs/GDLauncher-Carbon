import { Button } from "@gd/ui";
import { useNavigate, useRouteData } from "@solidjs/router";
import DoorImage from "/assets/images/door.png";
import { createEffect, createSignal, onCleanup, Show } from "solid-js";
import { parseTwoDigitNumber } from "@/utils/helpers";
import { Setter } from "solid-js";
import { DeviceCode } from "@/components/CodeInput";
import { createNotification } from "@gd/ui";
import { Trans } from "@gd/i18n";
import { rspc } from "@/utils/rspcClient";
import fetchData from "./auth.login.data";
import { handleStatus } from "@/utils/login";
interface Props {
  deviceCodeObject: any | null;
  setDeviceCodeObject: Setter<any>;
}

const CodeStep = (props: Props) => {
  const routeData: ReturnType<typeof fetchData> = useRouteData();
  const navigate = useNavigate();
  const [error, setError] = createSignal("");

  let cancelMutation = rspc.createMutation(["account.enroll.cancel"], {
    onError(error) {
      setError(error.message);
    },
  });

  let mutation = rspc.createMutation(["account.enroll.begin"], {
    onError(error) {
      setError(error.message);
    },
  });

  const handleRefersh = async () => {
    cancelMutation.mutate(null);
    mutation.mutate(null);
    if (routeData.isSuccess) {
      handleStatus(routeData, {
        onPolling: (info) => {
          props.setDeviceCodeObject({
            userCode: info.user_code,
            link: info.verification_uri,
            expiresAt: info.expires_at,
          });
        },
        onFail(error) {
          setError(error);
        },
      });
    }
  };

  const userCode = () => props.deviceCodeObject?.userCode;
  const oldUserCode = () => props.deviceCodeObject?.userCode;
  const deviceCodeLink = () => props.deviceCodeObject?.link;
  const expiresAt = () => props.deviceCodeObject?.expiresAt;
  const expiresAtFormat = () => new Date(expiresAt())?.getTime();
  const expiresAtMs = () => expiresAtFormat() - Date.now();
  const minutes = () =>
    Math.floor((expiresAtMs() % (1000 * 60 * 60)) / (1000 * 60));
  const seconds = () => Math.floor((expiresAtMs() % (1000 * 60)) / 1000);
  const [countDown, setCountDown] = createSignal(
    `${minutes()}:${parseTwoDigitNumber(seconds())}`
  );
  const [expired, setExpired] = createSignal(false);

  const resetCountDown = () => {
    setExpired(false);
    setCountDown(`${minutes()}:${parseTwoDigitNumber(seconds())}`);
  };

  const updateExpireTime = () => {
    if (minutes() <= 0 && seconds() <= 0) {
      setExpired(true);
    } else {
      resetCountDown();
    }
  };

  let interval: ReturnType<typeof setTimeout>;
  // let finalize = rspc.createQuery(() => ["account.enroll.finalize", null]);

  createEffect(() => {
    if (routeData.isSuccess) {
      handleStatus(routeData, {
        onComplete(_accountEntry) {
          // FINALIZE
          navigate("/library");
        },
      });
    }
  });

  createEffect(() => {
    if (expired()) {
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

  const [addNotification] = createNotification();

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
            <p class="mt-2 mb-0 text-[#E54B4B]">
              <Trans
                key="code_expired_message"
                options={{
                  defaultValue: "The code has been expired",
                }}
              />
            </p>
          </Show>
        </div>
        <Show when={!expired()}>
          <p class="mb-0 text-shade-0 mt-4">
            <span class="text-white mr-2">{countDown()}</span>
            <Trans
              key="before_expiring"
              options={{
                defaultValue: "before the code expires",
              }}
            />
          </p>
          <p class="text-shade-0 mb-0">
            <Trans
              key="enter_code_in_browser"
              options={{
                defaultValue:
                  "Enter the specified code on the browser page to complete the authorization",
              }}
            />
          </p>
        </Show>
      </div>
      <Show when={error()}>
        <p class="text-red m-0">{error()}</p>
      </Show>
      <Show when={!expired()}>
        <Button
          class="normal-case"
          onClick={() => {
            window.openExternalLink(deviceCodeLink() || "");
          }}
        >
          <Trans
            key="insert_code"
            options={{
              defaultValue: "Insert Code",
            }}
          />
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
              key="refresh"
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
