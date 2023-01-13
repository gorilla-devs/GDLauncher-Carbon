import { Button } from "@gd/ui";
import { useNavigate } from "@solidjs/router";
import { createSignal, Setter } from "solid-js";
import Logo from "/assets/images/gdlauncher_vertical_logo.svg";
// Forgive me for I have sinned
import { version } from "@package_json";

interface Props {
  setStep: Setter<number>;
  setDeviceCodeObject: Setter<any>;
}

const Auth = (_props: Props) => {
  const navigate = useNavigate();
  const [loading, setLoading] = createSignal(false);

  const handleClick = async () => {
    // await login(({ userCode, link, expiresAt }) => {
    // props.setDeviceCodeObject({
    //   userCode: "AXDLE",
    //   link: "",
    //   expiresAt: 548559,
    // });
    // props.setStep(1);
    // });

    console.log("Loading");
    setLoading(true);
  };

  return (
    <div>
      <div class="absolute left-0 right-0 m-auto -top-15 flex flex-col justify-center items-center">
        <img class="w-40" src={Logo} />
        <p class="text-shade-0">v{version}</p>
      </div>
      <div class="text-center flex flex-col justify-center items-center">
        <Button
          id="auth-button"
          loading={loading()}
          onClick={() => handleClick()}
        >
          Sign in with microsoft
        </Button>
        <p class="max-w-90 text-sm text-shade-0">
          Sign in with your Microsoft Account. By doing so, you accept all our
          policies and terms stated below.
        </p>
        <ul class="flex gap-3 list-none p-0 mb-8 underline text-sm">
          <li
            class="cursor-pointer"
            onClick={() => navigate("?m=privacyPolicy")}
          >
            Privacy Policy
          </li>
          <li
            class="cursor-pointer"
            onClick={() => navigate("?m=termsAndConditions")}
          >
            Terms and Conditions
          </li>
          <li
            class="cursor-pointer"
            onClick={() => navigate("?m=acceptableUsePolicy")}
          >
            Acceptable Use Policy
          </li>
        </ul>
      </div>
    </div>
  );
};

export default Auth;
