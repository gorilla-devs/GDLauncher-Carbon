import Logo from "/assets/images/gdlauncher_vertical_logo.svg";
import ModalLayout from "../../ModalLayout";
import { ModalProps, useModal } from "../..";
import { Trans } from "@gd/i18n";
import { Button } from "@gd/ui";

const OnBoarding = (props: ModalProps) => {
  const modalsContext = useModal();

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      overflowHiddenDisabled={true}
    >
      <div class="flex flex-col items-center justify-around w-120 pt-20 h-90">
        <div class="absolute left-0 right-0 flex justify-center items-center flex-col m-auto -top-15">
          <img class="w-40" src={Logo} />
        </div>
        <div class="absolute right-5 top-5">
          <div
            class="i-ri:close-fill text-2xl text-shade-0 cursor-pointer"
            onClick={() => modalsContext?.closeModal()}
          />
        </div>
        <div class="flex flex-col">
          <h2 class="text-center font-normal text-sm">
            <Trans
              key="onboarding.welcome_gdlauncher_title"
              options={{
                defaultValue: "Welcome to GDLauncher",
              }}
            />
          </h2>
          <p class="text-center text-shade-0 leading-6 mb-8">
            <Trans
              key="onboarding.welcome_gdlauncher_text"
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
              key="onboarding.add_instance"
              options={{
                defaultValue: "+ Add Instance",
              }}
            />
          </Button>

          <div class="flex items-center gap-2 cursor-pointer transition ease-in-out text-accent hover:text-primary">
            <div class="text-2xl i-ri:download-2-line" />
            <Trans
              key="onboarding.import_instance_or_zip"
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
