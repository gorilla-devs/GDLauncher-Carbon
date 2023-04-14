import { useNavigate, useRouteData } from "@solidjs/router";
import { createEffect, createSignal, Setter, Show } from "solid-js";
import Logo from "/assets/images/gdlauncher_vertical_logo.svg";
import { Trans } from "@gd/i18n";
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
          <Trans
            key="login.sign_in_with_microsoft"
            options={{
              defaultValue: "Sign in with microsoft",
            }}
          />
        </Button>
        <p class="text-shade-0 max-w-90 text-sm">
          <Trans
            key="login.sign_in_with_microsoft_text"
            options={{
              defaultValue:
                "Sign in with your Microsoft Account. By doing so, you accept all our policies and terms stated below.",
            }}
          />
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
            <Trans
              key="login.privacy_policy"
              options={{
                defaultValue: "Privacy Policy",
              }}
            />
          </li>
          <li
            class="cursor-pointer"
            onClick={() => {
              modalsContext?.openModal({
                name: "termsAndConditions",
              });
            }}
          >
            <Trans
              key="login.terms_and_conditions"
              options={{
                defaultValue: "Terms and Conditions",
              }}
            />
          </li>
          <li
            class="cursor-pointer"
            onClick={() =>
              modalsContext?.openModal({ name: "acceptableUsePolicy" })
            }
          >
            <Trans
              key="login.acceptable_use_policy"
              options={{
                defaultValue: "Acceptable Use Policy",
              }}
            />
          </li>
        </ul>
      </div>
    </div>
  );
};

export default Auth;
