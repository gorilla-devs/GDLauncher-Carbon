/* eslint-disable i18next/no-literal-string */
import { useNavigate } from "@solidjs/router";
import { createEffect, createSignal, Setter, Show } from "solid-js";
import Logo from "/assets/images/gdlauncher_vertical_logo.svg";
import { useTransContext } from "@gd/i18n";
import { queryClient, rspc } from "@/utils/rspcClient";
import { useModal } from "@/ModalsManager";
import { Button } from "@gd/ui";

type Props = {
  setStep: Setter<number>;
  setDeviceCodeObject: Setter<any>;
};

type LoginRes = {
  PollingCode: PollingCode;
};

type PollingCode = {
  user_code: string;
  verification_uri: string;
  expires_at: Date;
};

const Auth = (props: Props) => {
  const [t] = useTransContext();
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal(false);
  const modalsContext = useModal();

  let mutation = rspc.createMutation(["account.enroll.begin"], {
    onError() {
      setError(true);
    },
  });

  let cancelMutation = rspc.createMutation(["account.enroll.cancel"], {});

  let status = rspc.createQuery(() => ["account.enroll.getStatus", null]);

  const handleClick = async () => {
    mutation.mutate(null);

    if (status.isSuccess) {
      const data = status.data;
      if ((data as any).PollingCode) {
        const info = (data as any).PollingCode;

        if (info) {
          console.log("YESY", data, info);
          props.setDeviceCodeObject({
            userCode: info.user_code,
            link: info.verification_uri,
            expiresAt: info.expires_at,
          });
          props.setStep(1);
        }
      } else if ((data as any).Failed) {
        cancelMutation.mutate(null);
      }

      // console.log("Loading");
      setLoading(true);
    }
  };

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
          loading={loading()}
          size="large"
          onClick={() => handleClick()}
        >
          {t("sign_in_with_microsoft") || ""}
        </Button>
        <p class="text-shade-0 max-w-90 text-sm">
          {t("sign_in_with_microsoft_text")}
        </p>
        <Show when={error()}>
          <p class="text-red">Error</p>
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
