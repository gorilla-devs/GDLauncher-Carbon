import { Button } from "@gd/ui";
import Logo from "../../../assets/images/gdlauncher_vertical_logo.svg";
import { setStep } from "./Login";

const Auth = () => {
  return (
    <div>
      <div class="absolute left-0 right-0 m-auto -top-15 flex flex-col justify-center items-center">
        <img class="w-40" src={Logo} />
        <p class="text-[#8A8B8F]">v1.1.26</p>
      </div>
      <div class="text-center flex flex-col justify-center items-center">
        <Button
          onClick={() => {
            setStep(1);
          }}
        >
          Sign in with microsoft
        </Button>
        <p class="max-w-90 text-sm text-[#8A8B8F]">
          Sign in with your Microsoft Account. By doing so, you accept all our
          policies and terms stated below.
        </p>
        <ul class="flex gap-3 list-none p-0 mb-8 underline">
          <li>Privacy Policy</li>
          <li>Terms and Conditions</li>
          <li>Acceptable Use Policy</li>
        </ul>
      </div>
    </div>
  );
};

export default Auth;
