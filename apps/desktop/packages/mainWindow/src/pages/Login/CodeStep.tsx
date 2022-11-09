import { Button } from "@gd/ui";
import { useNavigate } from "@solidjs/router";
import { DeviceCodeObject } from "@gd/core";
import DoorImage from "../../../assets/images/door.png";

type Props = {
  deviceCodeObject: DeviceCodeObject | null;
};

const CodeStep = (props: Props) => {
  const navigate = useNavigate();

  const userCode = () => props.deviceCodeObject?.userCode;
  const deviceCodeLink = () => props.deviceCodeObject?.link;

  return (
    <div class="flex flex-col justify-between items-center gap-5 p-10 text-center">
      <img src={DoorImage} />
      <div>
        <input value={userCode()} />
        <p class="text-[#8A8B8F]">
          Enter the specified code on the browser page to complete the
          authorization
        </p>
      </div>
      <Button
        onClick={() => {
          // navigate("/home");
          window.openExternalLink(deviceCodeLink() || "");
        }}
      >
        Insert the code
      </Button>
    </div>
  );
};

export default CodeStep;
