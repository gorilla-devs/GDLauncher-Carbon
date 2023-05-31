import ModalLayout from "../../ModalLayout";
import { ModalProps } from "../..";
import { Steps } from "@gd/ui";
import { Match, Switch, createSignal } from "solid-js";
import SecondStep from "./SecondStep";
import ThirdStep from "./ThirdStep";
import FirstStep from "./FirstStep";
import { useTransContext } from "@gd/i18n";

const OnBoarding = (props: ModalProps) => {
  const [t] = useTransContext();

  const onBoardingSteps = [
    { label: t("introduction"), icon: <div>1</div> },
    { label: t("handle_java"), icon: <div>2</div> },
    { label: t("import_instances"), icon: <div>3</div> },
  ];
  const [currentStep, setCurrentStep] = createSignal(0);

  const nextStep = () => {
    setCurrentStep((prev) => prev + 1);
  };

  return (
    <ModalLayout
      preventClose
      noHeader={props.noHeader}
      title={props?.title}
      overflowHiddenDisabled={true}
    >
      <div class="select-none">
        <div
          class="max-w-70 mx-auto"
          classList={{
            hidden: currentStep() === 2,
          }}
        >
          <Steps steps={onBoardingSteps} currentStep={currentStep()} />
        </div>
        <div class="absolute right-5 top-5">
          <div
            class="i-ri:close-fill text-2xl text-darkSlate-50 cursor-pointer"
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
          <p class="text-center text-darkSlate-50 leading-6 mb-8">
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
            type="outline"
            style={{ width: "100%", "max-width": "200px" }}
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
    </ModalLayout>
  );
};

export default OnBoarding;
