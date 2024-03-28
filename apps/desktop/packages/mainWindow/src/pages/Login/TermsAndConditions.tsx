import { Show, createSignal } from "solid-js";
import { Trans } from "@gd/i18n";
import { Button, Checkbox, createNotification } from "@gd/ui";
import { useModal } from "@/managers/ModalsManager";
import { rspc } from "@/utils/rspcClient";

type Props = {
  nextStep: () => void;
};

const TermsAndConditions = (props: Props) => {
  const [acceptedTOS, setAcceptedTOS] = createSignal(false);
  const [acceptedMetrics, setAcceptedMetrics] = createSignal(false);
  const [loadingButton, setLoadingButton] = createSignal(false);
  const modalsContext = useModal();
  const addNotification = createNotification();

  const activeUuid = rspc.createQuery(() => ({
    queryKey: ["account.getActiveUuid"]
  }));
  const settingsMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setSettings"]
  }));

  return (
    <div class="flex flex-col justify-between items-center text-center h-full box-border pb-4 pt-5 px-6 max-w-full">
      <div class="flex flex-col justify-between items-center w-full">
        <div class="flex flex-col gap-4 w-full">
          <div class="flex justify-between">
            <h2 class="m-0">
              <Trans key="login.we_value_privacy_title" />
              <Show when={activeUuid?.data}>
                {" - "}
                <Trans key="login.renew" />
              </Show>
            </h2>
          </div>
          <div class="overflow-y-scroll max-h-38">
            <div class="flex flex-col m-0 text-lightSlate-900 text-left gap-4 leading-5">
              <Show when={activeUuid?.data}>
                <div>
                  <Trans key="login.we_value_privacy_text_renew" />
                </div>
              </Show>
              <div>
                <Trans key="login.we_value_privacy_text1" />
              </div>
              <div>
                <Trans key="login.we_value_privacy_text2" />
              </div>
              <div>
                <Trans key="login.we_value_privacy_text3" />
              </div>
              <div>
                <Trans key="login.we_value_privacy_text4" />
              </div>
            </div>
          </div>
        </div>
      </div>
      <div class="w-full flex flex-col items-center p-4">
        <div class="flex justify-between items-center w-full">
          <div class="flex flex-col gap-2">
            <div class="flex gap-2">
              <Checkbox
                checked={acceptedMetrics()}
                onChange={() => {
                  setAcceptedMetrics((prev) => !prev);
                }}
              />
              <p class="m-0 text-lightSlate-400 leading-5 text-xs select-none text-left">
                <Trans key="login.cookies_tracking" />
              </p>
            </div>
            <div class="flex gap-2">
              <Checkbox
                checked={acceptedTOS()}
                onChange={() => {
                  setAcceptedTOS((prev) => !prev);
                }}
              />
              <p class="m-0 text-lightSlate-400 leading-5 text-xs select-none">
                <Trans key="login.read_and_accept">
                  I have read and accept
                  <span
                    class="cursor-pointer underline text-lightSlate-50"
                    onClick={() => {
                      modalsContext?.openModal({
                        name: "termsAndConditions"
                      });
                    }}
                  >
                    Terms
                  </span>
                  and
                  <span
                    class="underline text-lightSlate-50 cursor-pointer"
                    onClick={() => {
                      modalsContext?.openModal({
                        name: "privacyStatement"
                      });
                    }}
                  >
                    Privacy Policy
                  </span>
                </Trans>
              </p>
            </div>
          </div>
          <div class="flex flex-col gap-4">
            <div
              class="text-xs underline whitespace-nowrap text-lightSlate-50 cursor-pointer"
              onClick={() => {
                window?.openCMPWindow();
              }}
            >
              <Trans key="login.manage_cmp" />
            </div>
            <Button
              variant="primary"
              size="large"
              disabled={!acceptedTOS()}
              loading={loadingButton()}
              onClick={async () => {
                setLoadingButton(true);
                try {
                  await settingsMutation.mutateAsync({
                    termsAndPrivacyAccepted: {
                      Set: true
                    },
                    metricsEnabled: {
                      Set: acceptedMetrics()
                    }
                  });

                  props.nextStep();
                } catch (err) {
                  console.log(err);
                  addNotification("Error during consent saving", "error");
                }

                setLoadingButton(false);
              }}
            >
              <Trans key="login.next" />
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
};

export default TermsAndConditions;
