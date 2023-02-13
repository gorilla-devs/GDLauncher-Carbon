/* eslint-disable i18next/no-literal-string */
import { Trans } from "@gd/i18n";
import { StepsProps } from ".";
import JavaLogo from "/assets/images/icons/java-logo.svg";
import { Button } from "@gd/ui";

const firstStep = (props: StepsProps) => {
  return (
    <div class="flex flex-col w-full h-full">
      <div class="flex flex-col items-center">
        <img src={JavaLogo} class="h-16 w-16" />
        <h3>Java 8 missing</h3>
      </div>
      <p class="text-center text-shade-3">
        <Trans
          key="missing_java_text"
          options={{
            defaultValue:
              "For an optimal experience, we sugges letting us take care of java for you. Only manually manage java if you know what yur're doing, it may result in GDLauncher not working!",
          }}
        />
      </p>
      <button
        onClick={() => {
          props?.nextStep?.();
        }}
      >
        NEXT
      </button>
      <div class="w-full flex">
        <Button rounded variant="secondary" size="large">
          <Trans
            key="manual_setup"
            options={{
              defaultValue: "Manual setup",
            }}
          />
        </Button>
        <Button rounded size="large">
          <Trans
            key="automatic_setup"
            options={{
              defaultValue: "Automatic setup",
            }}
          />
        </Button>
      </div>
    </div>
  );
};

export default firstStep;
