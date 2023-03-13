import Logo from "/assets/images/gdlauncher_vertical_logo.svg";
import ModalLayout from "../../ModalLayout";
import { ModalProps, useModal } from "../..";
import { Trans } from "@gd/i18n";
import { Button, Steps } from "@gd/ui";
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
        <div class="flex flex-col">
          <h2 class="text-center font-normal text-sm">
            <Trans
              key="welcome_gdlauncher_title"
              options={{
                defaultValue: "Welcome to GDLauncher",
              }}
            />
          </h2>
          <p class="text-center text-shade-0 leading-6 mb-8">
            <Trans
              key="welcome_gdlauncher_text"
              options={{
                defaultValue:
                  "To start enjoying your favorite game you will need to create an instance. You can do this by selecting one of the modpacks available or by importing a zip or an instance from another launcher on your computer",
              }}
            />
          </p>
        </div>
        <div class="flex flex-col items-center gap-6">
          <Button
            variant="outline"
            style={{ width: "100%", "max-width": "200px" }}
          >
            <Trans
              key="add_instance"
              options={{
                defaultValue: "+ Add Instance",
              }}
            />
          </Button>

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
