import { Button } from "@gd/ui";
import { createSignal, Setter } from "solid-js";
import Logo from "/assets/images/gdlauncher_vertical_logo.svg";
import { useTransContext } from "@gd/i18n";
import { useModal } from "@/ModalsManager";
interface Props {
  setStep: Setter<number>;
  setDeviceCodeObject: Setter<any>;
}

const Auth = (_props: Props) => {
  const [t] = useTransContext();
  const [loading, setLoading] = createSignal(false);
  const modalsContext = useModal();

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

  console.log(__APP_VERSION__);

  return (
    <div>
      <div class="absolute left-0 right-0 flex flex-col justify-center items-center m-auto -top-15">
        <img class="w-40" src={Logo} />
        <p class="text-shade-0">{__APP_VERSION__}</p>
      </div>
      <div class="text-center flex flex-col justify-center items-center">
        <Button
          id="auth-button"
          loading={loading()}
          size="large"
          onClick={() => handleClick()}
        >
          {t("sign_in_with_microsoft") || ""}
        </Button>
        <p class="max-w-90 text-sm text-shade-0">
          {t("sign_in_with_microsoft_text")}
        </p>
        <ul class="flex gap-3 list-none p-0 text-sm mb-8 underline">
          <li
            class="cursor-pointer"
            onClick={() => {
              modalsContext?.openModal({ name: "privacyPolicy" });
            }}
          >
            {t("privacy_policy")}
          </li>
          <li
            class="cursor-pointer"
            onClick={() => {
              modalsContext?.openModal({
                name: "termsAndConditions",
                url: "/library/DDAEDF",
              });
            }}
          >
            {t("terms_and_conditions")}
          </li>
          <li
            class="cursor-pointer"
            onClick={() =>
              modalsContext?.openModal({ name: "acceptableUsePolicy" })
            }
          >
            {t("acceptable_use_policy")}
          </li>
        </ul>
      </div>
    </div>
  );
};

export default Auth;
