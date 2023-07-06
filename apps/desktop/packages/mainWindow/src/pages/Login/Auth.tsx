import { useRouteData } from "@solidjs/router";
import { createSignal, onMount, Show } from "solid-js";
import { Trans } from "@gd/i18n";
import { rspc } from "@/utils/rspcClient";
import { Button } from "@gd/ui";
import fetchData from "./auth.login.data";
import { trackEvent } from "@/utils/analytics";

const Auth = () => {
  const [error, setError] = createSignal<null | string>(null);
  const [clicked, setClicked] = createSignal(false);
  const [retry, setRetry] = createSignal(0);
  const routeData: ReturnType<typeof fetchData> = useRouteData();

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
      if (!routeData.status.data) {
        console.log("TEST-1");
        accountEnrollCancelMutation.mutate(undefined);
      }
      accountEnrollBeginMutation.mutate(undefined);
      setRetry((prev) => prev + 1);
    }
    if (retry() > 3) {
      setError("Something went wrong while logging in, try again!");
      if (routeData.status.data) {
        console.log("TEST-2");

        accountEnrollCancelMutation.mutate(undefined);
      }
      setClicked(false);
    }
  };

  const handleClick = async () => {
    trackEvent("microsoft_auth");
    setClicked(true);
    if (!routeData.status.data) {
      accountEnrollBeginMutation.mutate(undefined);
    } else {
      console.log("TEST-3");
      accountEnrollCancelMutation.mutate(undefined);
      accountEnrollBeginMutation.mutate(undefined);
    }
  };

  return (
    <div class="flex flex-col justify-center items-center text-center">
      <Button
        class="mb-12"
        loading={clicked()}
        size="large"
        onClick={() => handleClick()}
      >
        <Trans
          key="login.sign_in_with_microsoft"
          options={{
            defaultValue: "Sign in with Microsoft",
          }}
        />
      </Button>
      <p class="text-darkSlate-50 text-sm max-w-90 mb-10">
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
