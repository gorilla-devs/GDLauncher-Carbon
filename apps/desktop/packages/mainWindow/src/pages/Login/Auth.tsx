import { useNavigate, useRouteData } from "@solidjs/router";
import { createEffect, createSignal, Setter, Show } from "solid-js";
import { Trans } from "@gd/i18n";
import { rspc } from "@/utils/rspcClient";
import { Button } from "@gd/ui";
import fetchData from "./auth.login.data";
import { handleStatus } from "@/utils/login";
import { DeviceCodeObjectType } from ".";
import { trackEvent } from "@/utils/analytics";

type Props = {
  nextStep: () => void;
  setDeviceCodeObject: Setter<DeviceCodeObjectType>;
};

const Auth = (props: Props) => {
  const [enrollmentInProgress, setEnrollmentInProgress] = createSignal(false);
  const [error, setError] = createSignal<null | string>(null);
  const [clicked, setClicked] = createSignal(false);
  const [retry, setRetry] = createSignal(0);
  const navigate = useNavigate();
  const routeData: ReturnType<typeof fetchData> = useRouteData();

  const accountEnrollFinalizeMutation = rspc.createMutation([
    "account.enroll.finalize",
  ]);

  const accountEnrollCancelMutation = rspc.createMutation([
    "account.enroll.cancel",
  ]);

  const accountEnrollBeginMutation = rspc.createMutation(
    ["account.enroll.begin"],
    {
      onError() {
        retryLogin();
      },
    }
  );

  const retryLogin = () => {
    while (retry() <= 3) {
      if (enrollmentInProgress()) {
        accountEnrollCancelMutation.mutate(undefined);
      }
      accountEnrollBeginMutation.mutate(undefined);
      setRetry((prev) => prev + 1);
    }
    if (retry() > 3) {
      setError("Something went wrong while logging in, try again!");
      if (enrollmentInProgress()) {
        accountEnrollCancelMutation.mutate(undefined);
      }
      setEnrollmentInProgress(false);
      setClicked(false);
    }
  };

  const handleClick = async () => {
    trackEvent("microsoft_auth");
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
        props.nextStep();
      },
      onFail() {
        retryLogin();
        setError("Something went wrong while logging in, Try again!");
      },
      onError(_error) {
        setError("Something went wrong while logging in, Try again!");
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
    <div class="flex flex-col justify-center items-center text-center mb-6">
      <Button loading={clicked()} size="large" onClick={() => handleClick()}>
        <Trans
          key="login.sign_in_with_microsoft"
          options={{
            defaultValue: "Sign in with Microsoft",
          }}
        />
      </Button>
      <p class="text-darkSlate-50 text-sm max-w-90">
        <Trans
          key="login.sign_in_with_microsoft_text"
          options={{
            defaultValue:
              "Sign in with your Microsoft Account. By doing so, you accept all our policies and terms stated below.",
          }}
        />
      </p>
      <Show when={error()}>
        <p class="m-0 text-red-500">{error()}</p>
      </Show>
    </div>
  );
};

export default Auth;
