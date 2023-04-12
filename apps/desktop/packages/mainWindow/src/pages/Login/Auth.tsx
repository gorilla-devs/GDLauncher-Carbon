import { useNavigate, useRouteData } from "@solidjs/router";
import { createEffect, createSignal, Setter, Show } from "solid-js";
import Logo from "/assets/images/gdlauncher_vertical_logo.svg";
import { useTransContext } from "@gd/i18n";
import { rspc } from "@/utils/rspcClient";
import { Button } from "@gd/ui";
import fetchData from "./auth.login.data";
import { handleStatus } from "@/utils/login";
import { useModal } from "@/managers/ModalsManager";
import { DeviceCodeObjectType } from ".";

type Props = {
  setStep: Setter<number>;
  setDeviceCodeObject: Setter<DeviceCodeObjectType>;
};

const Auth = (props: Props) => {
  const [t] = useTransContext();
  const [enrollmentInProgress, setEnrollmentInProgress] = createSignal(false);
  const [error, setError] = createSignal<null | string>(null);
  const [clicked, setClicked] = createSignal(false);
  const navigate = useNavigate();
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  const modalsContext = useModal();

  const accountEnrollBeginMutation = rspc.createMutation(
    ["account.enroll.begin"],
    {
      onError(error) {
        setError(error.message);
      },
    }
  );

  const accountEnrollFinalizeMutation = rspc.createMutation([
    "account.enroll.finalize",
  ]);

  const accountEnrollCancelMutation = rspc.createMutation([
    "account.enroll.cancel",
  ]);

  const handleClick = async () => {
    setClicked(true);
    if (!routeData.status.data || !enrollmentInProgress()) {
      accountEnrollBeginMutation.mutate(undefined);
    } else {
      accountEnrollCancelMutation.mutate(undefined);
      accountEnrollBeginMutation.mutate(undefined);
    }
  };

  createEffect(() => {
    if (routeData.status.isSuccess && !routeData.status.data) {
      setEnrollmentInProgress(false);
    } else {
      setEnrollmentInProgress(true);
    }

    handleStatus(routeData.status, {
      onPolling: (info) => {
        setEnrollmentInProgress(true);
        setError(null);
        props.setDeviceCodeObject({
          userCode: info.userCode,
          link: info.verificationUri,
          expiresAt: info.expiresAt,
        });
        props.setStep(1);
      },
      onFail() {
        setEnrollmentInProgress(false);
        setError("something went wrong while logging in");
        accountEnrollCancelMutation.mutate(undefined);
      },
      onComplete() {
        setError(null);
        if (enrollmentInProgress()) {
          accountEnrollFinalizeMutation.mutate(undefined);
        }
        navigate("/library");
        setEnrollmentInProgress(false);
      },
    });
  });

  return (
    <div>
      <div class="absolute right-0 flex justify-center items-center flex-col left-0 m-auto -top-15">
        <img class="w-40" src={Logo} />
        <p class="text-shade-0">{__APP_VERSION__}</p>
      </div>
      <div class="flex flex-col justify-center items-center text-center">
        <Button
          id="auth-button"
          loading={clicked()}
          size="large"
          onClick={() => handleClick()}
        >
          {t("sign_in_with_microsoft") || ""}
        </Button>
        <p class="text-shade-0 text-sm max-w-90">
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
