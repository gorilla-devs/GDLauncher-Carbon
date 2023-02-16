import { Button } from "@gd/ui";
import { useNavigate } from "@solidjs/router";
import DoorImage from "/assets/images/door.png";
import { createEffect, createSignal, onCleanup, Show } from "solid-js";
// import { accounts, login } from "@/modules/components/accounts";
import { parseTwoDigitNumber } from "@/utils/helpers";
import { Setter } from "solid-js";
import { DeviceCode } from "@/components/CodeInput";
import { createNotification } from "@gd/ui";
import { Trans } from "@gd/i18n";
interface Props {
  deviceCodeObject: any | null;
  setDeviceCodeObject: Setter<any>;
}

const CodeStep = (props: Props) => {
  const navigate = useNavigate();

  const handleRefersh = async () => {
    // await login(({ userCode, link, expiresAt }) => {
    props.setDeviceCodeObject({
      userCode: "AXDLE",
      link: "",
      expiresAt: 548559,
    });
    // });
  };

  const userCode = () => props.deviceCodeObject?.userCode;
  const oldUserCode = () => props.deviceCodeObject?.userCode;
  const deviceCodeLink = () => props.deviceCodeObject?.link;
  const expiresAt = () => props.deviceCodeObject?.expiresAt || 0;
  const expiresAtMs = () => expiresAt() - Date.now();
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

  // eslint-disable-next-line no-undef
  let interval: NodeJS.Timer;

  createEffect(() => {
    // if (accounts.selectedAccountId) {
    // TODO: save in a store the default / last page
    navigate("/library");
    // }
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
            <p class="mt-2 text-[#E54B4B] mb-0">
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
          <p class="mb-0 mt-2 text-shade-0">
            <span class="text-white">{countDown()}</span>
            <Trans
              key="before_expiring"
              options={{
                defaultValue: "before the code expires",
              }}
            />
          </p>
          <p class="text-shade-0">
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
            {" "}
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
