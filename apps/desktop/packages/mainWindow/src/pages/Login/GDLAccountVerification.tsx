import { useGDNavigate } from "@/managers/NavigationManager";
import { convertSecondsToHumanTime } from "@/utils/helpers";
import { rspc } from "@/utils/rspcClient";
import { Navigate } from "@solidjs/router";
import {
  createEffect,
  createSignal,
  Match,
  onCleanup,
  onMount,
  Switch
} from "solid-js";

interface Props {
  nextStep: () => void;
  prevStep: () => void;
  activeUuid: string | null | undefined;
  transitionToLibrary: () => void;
}

const GDLAccountVerification = (props: Props) => {
  const [cooldown, setCooldown] = createSignal(0);
  const [sentVisible, setSentVisible] = createSignal(false);

  const settingsMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setSettings"]
  }));

  const verified = rspc.createQuery(() => ({
    queryKey: ["account.getGdlAccount", props.activeUuid!],
    enabled: !!props.activeUuid
  }));

  const requestNewVerificationTokenMutation = rspc.createMutation(() => ({
    mutationKey: ["account.requestNewVerificationToken"]
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

  let cooldownInterval: ReturnType<typeof setInterval> | undefined;

  return (
    <>
      <Switch>
        <Match when={verified.data?.isEmailVerified}>
          <Navigate href="/library" />
        </Match>
        <Match when={!verified.data?.isEmailVerified}>
          <div class="flex-1 w-full text-center gap-5 flex flex-col justify-between items-center">
            <div class="p-10">
              <h1>Check your Email for a Verification Link</h1>
              <div
                onClick={async () => {
                  if (cooldownInterval) {
                    return;
                  }

                  if (!props.activeUuid) {
                    throw new Error("No active uuid");
                  }

                  try {
                    const status =
                      await requestNewVerificationTokenMutation.mutateAsync(
                        props.activeUuid
                      );

                    if (status.status === "failed" && status.value) {
                      setSentVisible(false);

                      clearInterval(cooldownInterval);
                      cooldownInterval = undefined;

                      setCooldown(status.value);

                      cooldownInterval = setInterval(() => {
                        setCooldown((prev) => prev - 1);

                        if (cooldown() <= 0) {
                          setCooldown(0);
                          clearInterval(cooldownInterval);
                          cooldownInterval = undefined;
                        }
                      }, 1000);
                    } else if (status.status === "success") {
                      setSentVisible(true);
                      setTimeout(() => setSentVisible(false), 10000);
                    }
                  } catch (e) {
                    console.error(e);
                  }
                }}
                class="underline transition-all duration-100 ease-in-out"
                classList={{
                  "text-lightSlate-400 hover:text-lightSlate-50": !cooldown(),
                  "text-lightSlate-900": !!cooldown()
                }}
              >
                Request a new verification link
              </div>
              <div class="text-sm mt-2">
                <Switch>
                  <Match when={sentVisible()}>
                    <div class="text-green-500">
                      An email has been sent to your email address
                    </div>
                  </Match>
                  <Match when={cooldown()}>
                    <div class="text-lightSlate-500">
                      You need to wait {convertSecondsToHumanTime(cooldown())}{" "}
                      to request a new verification link.
                    </div>
                  </Match>
                </Switch>
              </div>
            </div>

            <div
              onClick={() => {
                settingsMutation.mutate({
                  hasCompletedGdlAccountSetup: {
                    Set: true
                  }
                });
                props.transitionToLibrary?.();
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
