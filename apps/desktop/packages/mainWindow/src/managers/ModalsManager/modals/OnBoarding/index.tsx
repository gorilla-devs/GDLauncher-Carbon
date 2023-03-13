import Logo from "/assets/images/gdlauncher_vertical_logo.svg";
import ModalLayout from "../../ModalLayout";
import { ModalProps, useModal } from "../..";
import { Trans } from "@gd/i18n";
import { Button, Radio, Steps } from "@gd/ui";
import { createSignal } from "solid-js";

const OnBoarding = (props: ModalProps) => {
  const modalsContext = useModal();
  const [steps, setSteps] = createSignal(0);

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      noOverflowHidden={true}
    >
      <div class="w-120 h-90 flex flex-col items-center justify-around">
        <Steps
          class="max-w-56"
          steps={[
            { label: "User type" },
            { label: "Instance" },
            { label: "Import" },
          ]}
          currentStep={1}
        />
        <div class="flex flex-col mt-4">
          <h2 class="text-center mb-2 text-base">
            <Trans
              key="select_user_mode_title"
              options={{
                defaultValue: "Select your user type",
              }}
            />
          </h2>
          <p class="text-center text-shade-0 leading-6 m-0">
            <Trans
              key="select_user_mode_text"
              options={{
                defaultValue:
                  "The user type affects how many settings customization options you will have.",
              }}
            />
          </p>
        </div>
        <div class="flex flex-col items-center gap-6">
          <Radio.group onChange={(value) => {}}>
            <Radio name="user-mode" />
            <Radio name="user-mode" />
          </Radio.group>

          <div class="flex items-center gap-2 text-accent cursor-pointer hover:text-primary transition ease-in-out">
            <div class="i-ri:download-2-line text-2xl" />
            <Trans
              key="import_instance_or_zip"
              options={{
                defaultValue: "Import instance / Zip",
              }}
            />
          </div>
        </div>
      </div>
    </ModalLayout>
  );
};

export default OnBoarding;
