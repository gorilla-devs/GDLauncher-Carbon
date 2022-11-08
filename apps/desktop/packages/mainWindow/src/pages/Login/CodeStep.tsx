import { Button } from "@gd/ui";
import { useNavigate } from "@solidjs/router";
import DoorImage from "../../../assets/images/door.png";

const CodeStep = () => {
  const navigate = useNavigate();

  return (
    <div class="flex flex-col justify-between items-center gap-5 p-10 text-center">
      <img src={DoorImage} />
      <button
        onClick={() => {
          navigate("/home");
        }}
      >
        auth
      </button>
      <div>
        <input />
        <p class="text-[#8A8B8F]">
          Enter the specified code on the browser page to complete the
          authorization
        </p>
      </div>
      <Button>Insert the code</Button>
    </div>
  );
};

export default CodeStep;
