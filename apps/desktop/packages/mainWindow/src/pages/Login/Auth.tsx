import { useNavigate, useRouteData } from "@solidjs/router";
import { createEffect, createSignal, Setter, Show } from "solid-js";
import Logo from "/assets/images/gdlauncher_vertical_logo.svg";
import { useTransContext } from "@gd/i18n";
import { rspc } from "@/utils/rspcClient";
import { Button } from "@gd/ui";
import fetchData from "./auth.login.data";
import { handleStatus } from "@/utils/login";
import { useModal } from "@/ModalsManager";

type Props = {
  setStep: Setter<number>;
  setDeviceCodeObject: Setter<any>;
};

const Auth = (props: Props) => {
  const [t] = useTransContext();
  const [error, setError] = createSignal("");
  const [clicked, setClicked] = createSignal(false);
  const navigate = useNavigate();
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  const modalsContext = useModal();

  const mutation = rspc.createMutation(["account.enroll.begin"], {
    onError(error) {
      setError(error.message);
    },
  });

  const finalizeMutation = rspc.createMutation(["account.enroll.finalize"]);

  const handleClick = async () => {
    setClicked(true);
    mutation.mutate(null);
  };

  createEffect(() => {
    if (routeData.isSuccess && clicked()) {
      handleStatus(routeData, {
        onPolling: (info) => {
          setError("");
          props.setDeviceCodeObject({
            userCode: info.user_code,
            link: info.verification_uri,
            expiresAt: info.expires_at,
          });
          props.setStep(1);
        },
        onFail() {
          setError("error");
        },
        onComplete() {
          setError("");
          finalizeMutation.mutate(null);
          navigate("/library");
        },
      });
    }
  });

  return (
    <div>
      <div class="absolute left-0 right-0 flex justify-center items-center flex-col m-auto -top-15">
        <img class="w-40" src={Logo} />
        <p class="text-shade-0">{__APP_VERSION__}</p>
      </div>
      <div class="flex flex-col justify-center items-center text-center">
        <Button
          id="auth-button"
          loading={routeData.isLoading && clicked()}
          size="large"
          onClick={() => handleClick()}
        >
          {t("sign_in_with_microsoft") || ""}
        </Button>
        <p class="text-shade-0 max-w-90 text-sm">
          {t("sign_in_with_microsoft_text")}
        </p>
        <Show when={error()}>
          <p class="m-0 text-red">{error()}</p>
        </Show>
        <ul class="flex text-sm list-none gap-3 p-0 mb-8 underline">
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
