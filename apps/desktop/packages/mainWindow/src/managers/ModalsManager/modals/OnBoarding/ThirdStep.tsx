import { useModal } from "../..";
import { Button } from "@gd/ui";
import { rspc } from "@/utils/rspcClient";
import { For, Match, Show, Switch, createSignal } from "solid-js";
import { ImportEntityStatus } from "@gd/core_module/bindings";
import EntityCard from "@/components/Card/EntityCard";
import SingleEntity from "./SingleEntity";

import CurseForgeLogo from "/assets/images/icons/curseforge_logo.svg";
import ATLauncherLogo from "/assets/images/icons/atlauncher_logo.svg";
import FTBLogo from "/assets/images/icons/ftb_logo.svg";
import MultiMCLogo from "/assets/images/icons/multimc_logo.png";
import TechnicLogo from "/assets/images/icons/technic_logo.svg";
import PrismLogo from "/assets/images/icons/prism_logo.svg";
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg";
import LegacyGDL from "/assets/images/icons/legacy_gdlauncher.svg";
import { Trans } from "@gd/i18n";
import { isDownloaded } from "./SingleImport";

interface Props {
  prevStep: () => void;
  isImportInstance?: boolean;
}
const ThirdStep = (props: Props) => {
  const modalsContext = useModal();
  const [entity, setEntity] = createSignal<ImportEntityStatus | undefined>();

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
    <div
      class={`flex flex-col items-center justify-between ${
        props.isImportInstance ? "w-full p-4" : "w-120 lg:w-160"
      } h-full box-border pt-6`}
    >
      <Switch>
        <Match when={entities.isLoading}>
          <div class="w-full h-full flex items-center justify-center">
            <div class="i-formkit:spinner animate-spin w-10 h-10 text-sky-800" />
          </div>
        </Match>
        <Match when={entity()}>
          <SingleEntity
            entity={entity() as ImportEntityStatus}
            setEntity={setEntity}
          />
        </Match>
        <Match when={!entity()}>
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
          <Show when={!props.isImportInstance}>
            <div class="w-full flex justify-between">
              <Button
                onClick={() => {
                  props.prevStep();
                }}
                size="large"
                type="secondary"
              >
                <Trans key="onboarding.prev" />
              </Button>
              <Button
                onClick={() => {
                  modalsContext?.closeModal();
                }}
                size="large"
                type="primary"
              >
                {isDownloaded() ? (
                  <Trans key="onboarding.done" />
                ) : (
                  <Trans key="onboarding.skip" />
                )}
              </Button>
            </div>
          </Show>
        </Match>
      </Switch>
    </div>
  );
};

export default ThirdStep;
