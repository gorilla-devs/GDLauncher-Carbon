import { Button, CodeInput } from "@gd/ui";
import { useNavigate } from "@solidjs/router";
import { DeviceCodeObject } from "@gd/core";
import DoorImage from "/assets/images/door.png";
import { createEffect } from "solid-js";
import { accounts } from "@/modules/components/accounts";

type Props = {
  deviceCodeObject: DeviceCodeObject | null;
};

const CodeStep = (props: Props) => {
  const navigate = useNavigate();

  const userCode = () => props.deviceCodeObject?.userCode;
  const deviceCodeLink = () => props.deviceCodeObject?.link;

  createEffect(() => {
    if (accounts.selectedAccountId) {
      console.log("selectedAccount", accounts.selectedAccountId);
      navigate("/home");
    }
  });

  return (
    <div class="flex flex-col justify-between items-center gap-5 p-10 text-center">
      <img src={DoorImage} />
      <div>
        <div class="flex justify-center">
          <CodeInput value={userCode() || ""} />
          <div class="i-gdl:copy w-4 h-4" />
        </div>
        <p class="text-[#8A8B8F]">
          Enter the specified code on the browser page to complete the
          authorization
        </p>
      </div>
      <Button
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
