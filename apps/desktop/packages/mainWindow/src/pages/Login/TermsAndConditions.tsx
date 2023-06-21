import { createSignal, Setter } from "solid-js";
import { Trans } from "@gd/i18n";
import { Button, Checkbox } from "@gd/ui";
import { useModal } from "@/managers/ModalsManager";

type Props = {
  setStep: Setter<number>;
};

const TermsAndConditions = (props: Props) => {
  const [accepted, setAccepted] = createSignal(false);
  const modalsContext = useModal();

  return (
    <div class="flex flex-col justify-between items-center text-center pb-4 pt-5 px-6 h-full">
      <div class="flex flex-col justify-between items-center w-full">
        <div class="flex flex-col gap-4">
          <div class="flex justify-between">
            <h2 class="m-0">
              <Trans key="login.we_value_privacy_title" />
            </h2>
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
          </div>
          <p class="m-0 text-darkSlate-100 leading-5 text-left">
            <Trans key="login.we_value_privacy_text" />
          </p>
        </div>
      </div>
      <div class="w-full flex flex-col items-center">
        <div class="flex justify-between items-center w-full">
          <div
            class="flex gap-2 cursor-pointer"
            // onClick={() => setAccepted((prev) => !prev)}
          >
            <Checkbox
              checked={accepted()}
              onChange={() => {
                setAccepted((prev) => !prev);
              }}
            />
            <p class="m-0 text-xs text-darkSlate-100 select-none leading-5">
              <Trans key="login.read_and_accept" />
            </p>
          </div>
          <Button
            variant="primary"
            size="small"
            disabled={!accepted()}
            onClick={() => {
              props.setStep((prev) => prev + 1);
            }}
          >
            <Trans key="login.next" />
          </Button>
        </div>
        <ul class="flex text-sm list-none gap-3 p-0 underline">
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

export default TermsAndConditions;
