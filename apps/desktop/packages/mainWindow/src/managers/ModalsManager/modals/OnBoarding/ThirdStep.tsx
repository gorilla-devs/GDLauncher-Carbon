import { Trans } from "@gd/i18n";
import { useModal } from "../..";
import Logo from "/assets/images/gdlauncher_vertical_logo.svg";
import { Button } from "@gd/ui";
import Import from "../InstanceCreation/Import";
import { rspc } from "@/utils/rspcClient";
import { Match, Switch, createEffect, createSignal } from "solid-js";

type Props = {
  prevStep: () => void;
};

const ThirdStep = (props: Props) => {
  const modalsContext = useModal();
  const [isLoading, setIsLoading] = createSignal(false);

  const legacyGDLauncherEntity = "legacygdlauncher";

  const instances = rspc.createQuery(() => [
    "instance.getImportableInstances",
    legacyGDLauncherEntity,
  ]);

  const scanImportableInstancesMutation = rspc.createMutation([
    "instance.scanImportableInstances",
  ]);

  createEffect(() => {
    scanImportableInstancesMutation.mutate(legacyGDLauncherEntity);
  });

  const CreateInstance = () => {
    return (
      <div class="flex flex-col items-center justify-between lg:w-160 h-full box-border">
        <div class="lg:w-[35rem]">
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
      </div>
    );
  };

  return (
    <div class="flex flex-col items-center justify-between w-120 h-120 lg:w-160 h-full box-border">
      <Switch>
        <Match when={instances.data && instances.data?.length > 0}>
          <div class="mt-10 lg:mt-20 h-full max-w-full">
            <Import setIsLoading={setIsLoading} />
          </div>
        </Match>
        <Match when={instances.data && instances.data?.length === 0}>
          <CreateInstance />
        </Match>
      </Switch>
      <div class="flex justify-between w-full">
        <Button
          disabled={isLoading()}
          type="secondary"
          size="large"
          onClick={() => {
            props.prevStep();
          }}
        >
          <Trans key="onboarding.prev" />
        </Button>
        <Button
          disabled={isLoading()}
          onClick={() => {
            modalsContext?.closeModal();
          }}
          size="large"
        >
          <Trans key="onboarding.start_playing" />
        </Button>
      </div>
    </div>
  );
};

export default ThirdStep;
