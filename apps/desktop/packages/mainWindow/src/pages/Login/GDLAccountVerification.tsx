import { useGDNavigate } from "@/managers/NavigationManager";
import { rspc } from "@/utils/rspcClient";
import { Navigate } from "@solidjs/router";
import { createEffect, Match, onCleanup, onMount, Switch } from "solid-js";

interface Props {
  nextStep: () => void;
  prevStep: () => void;
  activeUuid: string | null | undefined;
}

const GDLAccountVerification = (props: Props) => {
  const navigate = useGDNavigate();

  const verified = rspc.createQuery(() => ({
    queryKey: ["account.getGdlAccount", props.activeUuid!],
    enabled: !!props.activeUuid
  }));

  function invalidateEmailVerification() {
    verified.refetch();
  }

  let interval: ReturnType<typeof setInterval>;

  createEffect(async () => {
    if (props.activeUuid) {
      if (interval) {
        clearInterval(interval);
      }

      interval = setInterval(invalidateEmailVerification, 1000);
    }
  });

  onCleanup(() => {
    clearInterval(interval);
  });

  return (
    <>
      <Switch>
        <Match when={verified.data?.isEmailVerified}>
          <Navigate href="/library" />
        </Match>
        <Match when={!verified.data?.isEmailVerified}>
          <div class="flex-1 w-full text-center gap-5 flex flex-col justify-between items-center">
            <div class="p-10">
              <h1>Waiting for Verification</h1>
              <h3 class="text-lightSlate-400 mt-12">
                Check your email for a verification link
              </h3>
            </div>

            <div
              onClick={() => {
                navigate("/library");
              }}
              class="underline text-lightSlate-400 hover:text-lightSlate-50 transition-all duration-100 ease-in-out"
            >
              Verify Later
            </div>
          </div>
        </Match>
      </Switch>
    </>
  );
};

export default GDLAccountVerification;
