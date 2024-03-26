import { useModal } from "../..";
import { Button } from "@gd/ui";
import { rspc } from "@/utils/rspcClient";
import { For, Match, Show, Switch, createSignal } from "solid-js";
import { ImportEntityStatus } from "@gd/core_module/bindings";
import EntityCard from "@/components/Card/EntityCard";
import SingleEntity, { setInstances, setStep } from "./SingleEntity";

import { Trans } from "@gd/i18n";
import { isDownloaded } from "./SingleImport";
import { taskIds } from "@/utils/import";
import { ENTITIES } from "@/utils/constants";

interface Props {
  prevStep: () => void;
  isImportInstance?: boolean;
}

const [currentEntity, setCurrentEntity] = createSignal<
  ImportEntityStatus | undefined
>();

const ThirdStep = (props: Props) => {
  const modalsContext = useModal();

  const [entity, setEntity] = createSignal<ImportEntityStatus | undefined>();

  const entities = rspc.createQuery(() => ({
    queryKey: ["instance.getImportableEntities"]
  }));

  const handleClickEntity = (ent: ImportEntityStatus) => {
    if (ent.supported) {
      if (currentEntity() && !(currentEntity()?.entity === ent.entity)) {
        setStep("selectionStep");
        setInstances([]);
      }
      if (taskIds().every((x) => x === undefined)) {
        setStep("selectionStep");
        setInstances([]);
      }
      setEntity(ent);
      setCurrentEntity(ent);
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
          <div class="flex-1 w-full">
            <ul class="grid gap-2 p-0 grid-cols-3">
              <For
                each={entities.data?.sort(
                  (a, b) =>
                    (b.supported === true ? 1 : 0) -
                    (a.supported === true ? 1 : 0)
                )}
              >
                {(entity) => (
                  <EntityCard
                    entity={entity}
                    icon={ENTITIES[entity.entity].icon}
                    translation={ENTITIES[entity.entity].translation}
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
