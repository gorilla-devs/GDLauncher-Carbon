import { Button } from "@gd/ui";
import { StepsProps } from ".";
import { Trans } from "@gd/i18n";

const ManualStep = (props: StepsProps) => {
  return (
    <div class="w-110 h-75">
      <div class="flex flex-col justify-between w-full h-full">
        <div class="border-dashed border-2 border-primary">AAA</div>
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
