import { createSignal } from "solid-js";
import { Trans, useTransContext } from "@gd/i18n";
import CustomTrans from "../../components/i18n/Trans";
import { Button, Checkbox } from "@gd/ui";
import { useModal } from "@/managers/ModalsManager";
import { rspc } from "@/utils/rspcClient";

type Props = {
  nextStep: () => void;
};

const TermsAndConditions = (props: Props) => {
  const [accepted, setAccepted] = createSignal(false);
  const modalsContext = useModal();
  const [t] = useTransContext();

  const settingsMutation = rspc.createMutation(["settings.setSettings"]);

  return (
    <div class="flex flex-col justify-between items-center text-center pb-4 pt-5 px-6 h-full">
      <div class="flex flex-col justify-between items-center w-full">
        <div class="flex flex-col gap-4">
          <div class="flex justify-between">
            <h2 class="m-0">
              <Trans key="login.we_value_privacy_title" />
            </h2>
          </div>
          <p class="m-0 text-darkSlate-100 leading-5 text-left">
            <Trans key="login.we_value_privacy_text" />
          </p>
        </div>
      </div>
      <div class="w-full flex flex-col items-center">
        <div class="flex justify-between items-center w-full">
          <div class="flex gap-2">
            <Checkbox
              checked={accepted()}
              onChange={() => {
                setAccepted((prev) => !prev);
              }}
            />
            <p
              class="m-0 text-xs text-darkSlate-100 select-none leading-5"
              onClick={() => setAccepted((prev) => !prev)}
            >
              <Trans key="login.read_and_accept">
                I have read and accept
                <span
                  class="underline text-lightSlate-400 cursor-pointer"
                  onClick={() => {
                    modalsContext?.openModal({
                      name: "termsAndConditions",
                    });
                  }}
                >
                  Terms
                </span>
                and
                <span
                  class="underline text-lightSlate-400 cursor-pointer"
                  onClick={() => {
                    modalsContext?.openModal({
                      name: "acceptableUsePolicy",
                    });
                  }}
                >
                  Privacy Policy
                </span>
              </Trans>
            </p>
          </div>
          <Button
            type="secondary"
            size="small"
            rounded={false}
            onClick={() => {
              window?.openCMPWindow();
            }}
          >
            <Trans key="login.manage" />
          </Button>
          <Button
            variant="primary"
            size="small"
            disabled={!accepted()}
            onClick={() => {
              settingsMutation.mutate({ isLegalAccepted: true });
              props.nextStep();
            }}
          >
            <Trans key="login.next" />
          </Button>
        </div>
      </div>
    </div>
  );
};

export default TermsAndConditions;
