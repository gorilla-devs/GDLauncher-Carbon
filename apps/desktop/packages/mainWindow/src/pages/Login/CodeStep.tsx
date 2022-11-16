import { Button, CodeInput } from "@gd/ui";
import { useNavigate } from "@solidjs/router";
import { DeviceCodeObject } from "@gd/core";
import DoorImage from "/assets/images/door.png";
import { createEffect, createSignal, onCleanup, onMount } from "solid-js";
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
  const expiresAtMs = () => expiresAt() - Date.now();
  const minutes = () =>
    Math.floor((expiresAtMs() % (1000 * 60 * 60)) / (1000 * 60));
  const seconds = () => Math.floor((expiresAtMs() % (1000 * 60)) / 1000);
  const [countDown, setCountDown] = createSignal(
    `${minutes()}:${parseTwoDigitNumber(seconds())}`
  );

  const updateExpireTime = () => {
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
          <p class="mb-0 mt-4 text-[#8A8B8F]">
            <span class="text-white">{countDown()}</span> before the code
            expires
          </p>
        </div>
        <p class="text-[#8A8B8F]">
          Enter the specified code on the browser page to complete the
          authorization
        </p>
      </div>
      <Button
        class="normal-case"
        onClick={() => {
          window.openExternalLink(deviceCodeLink() || "");
        }}
      >
        Insert the code
      </Button>
    </div>
  );
};

export default CodeStep;
