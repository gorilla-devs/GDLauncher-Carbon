import { Trans } from "@gd/i18n";
import { useModal } from "../..";
import Logo from "/assets/images/gdlauncher_vertical_logo.svg";
import { Button } from "@gd/ui";

type Props = {
  prevStep: () => void;
};

const ThirdStep = (props: Props) => {
  const modalsContext = useModal();
  return (
    <div class="flex flex-col items-center justify-between w-160 h-140 box-border">
      <div class="w-[35rem]">
        <div class="flex justify-center items-center flex-col mt-20">
          <img class="w-50" src={Logo} />
        </div>
        <div class="absolute right-5 top-5">
          <div
            class="i-ri:close-fill text-2xl text-darkSlate-50 cursor-pointer"
            onClick={() => modalsContext?.closeModal()}
          />
        </div>
        <div class="flex flex-col mt-10">
          <p class="text-left text-darkSlate-50 leading-6 mb-8">
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
            type="outline"
            style={{ width: "100%", "max-width": "200px" }}
            onClick={() => {
              modalsContext?.closeModal();
              modalsContext?.openModal({
                name: "instanceCreation",
                url: "/modpacks",
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
          {/* <div class="flex items-center gap-2 cursor-pointer transition ease-in-out text-primary-300 hover:text-primary-500">
          <div class="text-2xl i-ri:download-2-line" />
          <Trans
          key="onboarding.import_instance_or_zip"
          options={{
            defaultValue: "Import instance / Zip",
          }}
        />
        </div> */}
        </div>
      </div>
      <div class="flex w-full align-between">
        <Button
          type="secondary"
          size="large"
          onClick={() => {
            props.prevStep();
          }}
        >
          <Trans key="onboarding.prev" />
        </Button>
      </div>
    </div>
  );
};

export default ThirdStep;
