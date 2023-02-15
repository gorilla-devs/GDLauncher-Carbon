import { Button } from "@gd/ui";
import { StepsProps } from ".";
import { Trans } from "@gd/i18n";

const ManualStep = (props: StepsProps) => {
  return (
    <div class="w-110 h-65">
      <div class="flex flex-col justify-between w-full h-full">
        <div class="border-dashed border-2 border-primary py-4 h-13 flex flex-col justify-center items-center">
          <div class="flex flex-col gap-2 justify-center items-center">
            <div class="i-ri:folder-open-fill text-shade-5 text-xl w-6" />
            <p class="m-0 text-shade-5">
              <Trans
                key="select_java_zip"
                options={{
                  defaultValue: "Select java {{version}} zip",
                  version: 8,
                }}
              />
            </p>
          </div>
        </div>
        <p class="text-shade-5 text-center">
          <Trans
            key="select_required_java_text"
            options={{
              defaultValue:
                "Select the required paths to java. Java 8 udes for all the versions < 1.17",
            }}
          />
        </p>
        <div class="w-full flex justify-between gap-4">
          <Button
            rounded
            variant="secondary"
            size="large"
            style={{ width: "100%", "max-width": "100%" }}
            onClick={() => {
              props.nextStep?.("intro");
            }}
          >
            <Trans
              key="back"
              options={{
                defaultValue: "Back",
              }}
            />
          </Button>
          <Button
            rounded
            size="large"
            style={{ width: "100%", "max-width": "100%" }}
            onClick={() => {
              props.nextStep?.("automatic");
            }}
          >
            <Trans
              key="setup"
              options={{
                defaultValue: "Setup",
              }}
            />
          </Button>
        </div>
      </div>
    </div>
  );
};

export default ManualStep;
