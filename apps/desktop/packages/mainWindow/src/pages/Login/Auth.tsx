/* eslint-disable i18next/no-literal-string */
import { useNavigate, useRouteData } from "@solidjs/router";
import { createEffect, createSignal, Setter, Show } from "solid-js";
import Logo from "/assets/images/gdlauncher_vertical_logo.svg";
import { useTransContext } from "@gd/i18n";
import { queryClient, rspc } from "@/utils/rspcClient";
import { Button } from "@gd/ui";
import fetchData from "./auth.login.data";

type Props = {
  setStep: Setter<number>;
  setDeviceCodeObject: Setter<any>;
};

const Auth = (props: Props) => {
  const [t] = useTransContext();
  const [error, setError] = createSignal("");
  const navigate = useNavigate();
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  let mutation = rspc.createMutation(["account.enroll.begin"], {
    onError(error) {
      setError(error.message);
    },
  });

  // let cancelMutation = rspc.createMutation(["account.enroll.finalize"], {});

  const handleClick = async () => {
    mutation.mutate(null);
  };

  createEffect(() => {
    if (routeData.isSuccess) {
      const data = routeData.data;
      if (typeof data === "string") return;
      if ("PollingCode" in data) {
        const info = data.PollingCode;

        if (info) {
          props.setDeviceCodeObject({
            userCode: info.user_code,
            link: info.verification_uri,
            expiresAt: info.expires_at,
          });
          props.setStep(1);
        }
      } else if ("Failed" in data) {
        const error = data.Failed;
        setError(error);
      } else if ("Complete" in data) {
        // navigate("/library");
      }
    }
  });

  console.log(__APP_VERSION__);

  return (
    <div>
      <div class="absolute left-0 right-0 flex flex-col justify-center items-center m-auto -top-15">
        <img class="w-40" src={Logo} />
        <p class="text-shade-0">{__APP_VERSION__}</p>
      </div>
      <div class="flex flex-col justify-center items-center text-center">
        <Button
          id="auth-button"
          loading={routeData.isLoading}
          size="large"
          onClick={() => handleClick()}
        >
          {t("sign_in_with_microsoft") || ""}
        </Button>
        <p class="text-shade-0 max-w-90 text-sm">
          {t("sign_in_with_microsoft_text")}
        </p>
        <Show when={error()}>
          <p class="text-red m-0">{error()} Error</p>
        </Show>
        <ul class="flex text-sm gap-3 list-none p-0 mb-8 underline">
          <li
            class="cursor-pointer"
            onClick={() => navigate("?m=privacyPolicy")}
          >
            {t("privacy_policy")}
          </li>
          <li
            class="cursor-pointer"
            onClick={() => navigate("?m=termsAndConditions")}
          >
            {t("terms_and_conditions")}
          </li>
          <li
            class="cursor-pointer"
            onClick={() => navigate("?m=acceptableUsePolicy")}
          >
            {t("acceptable_use_policy")}
          </li>
        </ul>
      </div>
    </div>
  );
};

export default Auth;
