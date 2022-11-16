import { Button, CodeInput } from "@gd/ui";
import { useNavigate } from "@solidjs/router";
import { DeviceCodeObject } from "@gd/core";
import DoorImage from "/assets/images/door.png";
import { createEffect, createSignal, onCleanup, onMount, Show } from "solid-js";
import { accounts } from "@/modules/components/accounts";
import { addNotification } from "@/notificationManager";
import { parseTwoDigitNumber } from "@/utils/helpers";

type Props = {
  deviceCodeObject: DeviceCodeObject | null;
};

const CodeStep = (props: Props) => {
  const navigate = useNavigate();

  const userCode = () => props.deviceCodeObject?.userCode;
  const deviceCodeLink = () => props.deviceCodeObject?.link;
  const expiresAt = () => props.deviceCodeObject?.expiresAt || 0;
  const expiresAtMs = () => 0;
  // const expiresAtMs = () => expiresAt() - Date.now();
  const minutes = () =>
    Math.floor((expiresAtMs() % (1000 * 60 * 60)) / (1000 * 60));
  const seconds = () => Math.floor((expiresAtMs() % (1000 * 60)) / 1000);
  const [countDown, setCountDown] = createSignal(
    `${minutes()}:${parseTwoDigitNumber(seconds())}`
  );
  const [expired, setExpired] = createSignal(false);

  const updateExpireTime = () => {
    console.log("TEST", minutes(), seconds());
    if (minutes() === 0 && seconds() === 0) {
      setExpired(true);
    }
    setCountDown(`${minutes()}:${parseTwoDigitNumber(seconds())}`);
  };

  const interval = () =>
    setInterval(() => {
      updateExpireTime();
    }, 1000);

  createEffect(() => {
    if (accounts.selectedAccountId) {
      console.log("selectedAccount", accounts.selectedAccountId);
      navigate("/home");
    }
  });

  onMount(() => {
    interval();
  });

  onCleanup(() => clearInterval(interval()));

  return (
    <div class="flex flex-col justify-between items-center gap-5 p-10 text-center">
      <img src={DoorImage} />
      <div>
        <div class="flex flex-col justify-center items-center">
          <CodeInput
            value={userCode() || ""}
            onClick={() => {
              window.copyToClipboard(userCode() || "");
              addNotification("The link has been copied");
            }}
          />
        </div>
        <Show when={!expired()}>
          <p class="mb-0 mt-2 text-[#8A8B8F]">
            <span class="text-white">{countDown()}</span> before the code
            expires
          </p>
          <p class="text-[#8A8B8F]">
            Enter the specified code on the browser page to complete the
            authorization
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
          Insert the code
        </Button>
      </Show>
      <Show when={expired()}>
        <p class="text-[#E54B4B] m-0">The code has been expired</p>
        <div class="flex">
          <span class="i-ri-refresh-line" />
          <h3 class="m-0">Refresh</h3>
        </div>
      </Show>
    </div>
  );
};

export default CodeStep;
