import { Trans } from "@gd/i18n";
import { useModal } from "../..";
import Logo from "/assets/images/gdlauncher_vertical_logo.svg";
import { Button, Steps } from "@gd/ui";
import { onBoardingSteps } from ".";

const ThirdStep = () => {
  const modalsContext = useModal();

  return (
    <div class="flex flex-col items-center justify-around w-120 pt-20 h-90">
      <div class="absolute left-0 right-0 flex justify-center items-center flex-col m-auto -top-15">
        <img class="w-40" src={Logo} />
      </div>
      <div class="w-full mt-4 max-w-70">
        <Steps steps={onBoardingSteps} currentStep={2} />
      </div>
      <div class="absolute right-5 top-5">
        <div
          class="i-ri:close-fill text-2xl text-darkSlate-50 cursor-pointer"
          onClick={() => modalsContext?.closeModal()}
        />
      </div>
      <div class="flex flex-col mt-10">
        <p class="text-center text-darkSlate-50 leading-6 mb-8">
          <Trans
            key="onboarding.import_instance_text"
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
          onClick={() => {
            modalsContext?.openModal({
              name: "instanceCreation",
              url: "/modpacks/",
            });
          }}
        >
          <Trans
            key="onboarding.add_instance"
            options={{
              defaultValue: "+ Add Instance",
            }}
          />
        </Button>

        <div class="flex items-center gap-2 cursor-pointer transition ease-in-out text-primary-300 hover:text-primary-500">
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
  );
};

export default ThirdStep;
