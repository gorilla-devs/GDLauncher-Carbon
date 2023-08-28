import { rspc } from "@/utils/rspcClient";
import { Trans } from "@gd/i18n";
import { Button, Switch } from "@gd/ui";

type Props = {
  nextStep: () => void;
  prevStep: () => void;
};

const SecondStep = (props: Props) => {
  let setSettingsMutation = rspc.createMutation(["settings.setSettings"]);
  let settingsQuery = rspc.createQuery(() => ["settings.getSettings"]);
  return (
    <div class="flex flex-col items-center justify-between lg:w-160 h-100 lg:h-140 box-border">
      <div class="flex flex-col h-full justify-center">
        <div class="flex items-center gap-4 w-100 lg:w-[35rem] mb-10">
          <p class="text-left text-darkSlate-100 m-0 font-normal w-fit leading-7">
            <Trans
              key="onboarding.java_title"
              options={{
                defaultValue:
                  "Do you want the launcher to automatically handle java for you? It will also download a managed java version if you don't have a correct one",
              }}
            />
          </p>
          <Switch
            checked={settingsQuery.data?.autoManageJava}
            onChange={(e) => {
              setSettingsMutation.mutate({ autoManageJava: e.target.checked });
            }}
          />
        </div>
      </div>
      <div class="flex justify-between w-full">
        <Button
          type="secondary"
          size="large"
          onClick={() => {
            props.prevStep();
          }}
        >
          <Trans key="onboarding.prev" />
        </Button>
        <Button
          onClick={() => {
            props.nextStep();
          }}
          size="large"
        >
          <Trans key="onboarding.next" />
        </Button>
      </div>
    </div>
  );
};

export default SecondStep;
