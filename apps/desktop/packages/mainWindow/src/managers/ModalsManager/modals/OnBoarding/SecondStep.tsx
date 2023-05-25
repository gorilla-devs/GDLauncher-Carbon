import { rspc } from "@/utils/rspcClient";
import { Trans } from "@gd/i18n";
import { Button } from "@gd/ui";

type Props = {
  nextStep: () => void;
};

const SecondStep = (props: Props) => {
  let setSettingsMutation = rspc.createMutation(["settings.setSettings"]);
  return (
    <div class="flex flex-col items-center justify-between w-120 h-80">
      <div class="flex flex-col h-full justify-center items-center">
        <p class="text-center font-normal max-w-100 text-darkSlate-100 m-0">
          <Trans
            key="onboarding.java_title"
            options={{
              defaultValue:
                "Do you want the launcher to automatically handle java for you? It will also download a managed java version if you don't have a correct one",
            }}
          />
        </p>
      </div>
      <div class="flex justify-between w-full">
        <Button
          variant="secondary"
          size="large"
          onClick={() => {
            setSettingsMutation.mutate({ autoManageJava: false });
            props.nextStep();
          }}
        >
          <Trans
            key="onboarding.java_no"
            options={{
              defaultValue: "No",
            }}
          />
        </Button>
        <Button
          onClick={() => {
            setSettingsMutation.mutate({ autoManageJava: true });
            props.nextStep();
          }}
          size="large"
        >
          <Trans
            key="onboarding.java_yes"
            options={{
              defaultValue: "Yes",
            }}
          />
        </Button>
      </div>
    </div>
  );
};

export default SecondStep;
