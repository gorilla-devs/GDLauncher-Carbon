import { Trans } from "@gd/i18n";
import { useModal } from "../..";
import Logo from "/assets/images/gdlauncher_vertical_logo.svg";
import { Button } from "@gd/ui";
import Import from "../InstanceCreation/Import";
import { rspc } from "@/utils/rspcClient";
import {
  For,
  Match,
  Show,
  Switch,
  createEffect,
  createSignal,
  onCleanup
} from "solid-js";
import { ImportEntityStatus } from "@gd/core_module/bindings";
import EntityCard from "@/components/Card/EntityCard";
import SingleEntity from "./SingleEntity";
import GdLauncherLogo from "/assets/images/gdlauncher_logo.svg";
import CurseForgeLogo from "/assets/images/icons/curseforge_logo.svg";
import ATLauncherLogo from "/assets/images/icons/atlauncher_logo.svg";
import FTBLogo from "/assets/images/icons/ftb_logo.svg";
import MultiMCLogo from "/assets/images/icons/multimc_logo.png";
import TechnicLogo from "/assets/images/icons/technic_logo.svg";
import PrismLogo from "/assets/images/icons/prism_logo.svg";
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg";
import LegacyGDL from "/assets/images/icons/legacy_gdlauncher.svg";

type Props = {
  prevStep: () => void;
};

const ThirdStep = (props: Props) => {
  const modalsContext = useModal();
  const [entity, setEntity] = createSignal<ImportEntityStatus | undefined>();
  const [isLoading, setIsLoading] = createSignal(false);

  const legacyGDLauncherEntity = "legacygdlauncher";

  // const instances = rspc.createQuery(() => [
  //   "instance.getImportableInstances",
  //   legacyGDLauncherEntity
  // ]);

  // const scanImportableInstancesMutation = rspc.createMutation([
  //   "instance.scanImportableInstances"
  // ]);

  // createEffect(() => {
  //   scanImportableInstancesMutation.mutate(legacyGDLauncherEntity);
  // });
  const entities = rspc.createQuery(() => ["instance.getImportableEntities"]);
  const icons = [
    LegacyGDL,
    CurseForgeLogo,
    ModrinthLogo,
    CurseForgeLogo,
    ModrinthLogo,
    ATLauncherLogo,
    TechnicLogo,
    FTBLogo,
    MultiMCLogo,
    PrismLogo
  ];
  const handleClickEntity = (entity: ImportEntityStatus) => {
    if (entity.supported) {
      setEntity(entity);
    }
  };
  return (
    <div class="flex flex-col items-center justify-between w-120 lg:w-160 h-full box-border pt-6">
      <Switch>
        <Match when={entities.isLoading}>
          <div>searching ...</div>
        </Match>
        <Match when={entity()}>
          <SingleEntity
            entity={entity() as ImportEntityStatus}
            setEntity={setEntity}
          />
        </Match>
        <Match when={!entity()}>
          <div class="w-full flex justify-end pt-5">
            <Button type="primary">Done</Button>
          </div>
          <div class=" flex-1 w-full">
            <ul class="grid grid-cols-3 gap-2 p-0">
              <For
                each={entities.data?.sort(
                  (a, b) =>
                    (b.supported === true ? 1 : 0) -
                    (a.supported === true ? 1 : 0)
                )}
              >
                {(entity, i) => (
                  <EntityCard
                    entity={entity}
                    icon={icons[i()]}
                    onClick={[handleClickEntity, entity]}
                  />
                )}
              </For>
            </ul>
          </div>
        </Match>
      </Switch>

      {/* <Switch>
        <Match when={instances.data && instances.data?.length > 0}>
          <div class="mt-10 h-full max-w-full">
            <Import setIsLoading={setIsLoading} />
          </div>
        </Match>
        <Match when={instances.data && instances.data?.length === 0}>
          <CreateInstance />
        </Match>
      </Switch> */}
      {/* <div class="flex justify-between w-full">
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
      </div> */}
    </div>
  );
};

export default ThirdStep;
