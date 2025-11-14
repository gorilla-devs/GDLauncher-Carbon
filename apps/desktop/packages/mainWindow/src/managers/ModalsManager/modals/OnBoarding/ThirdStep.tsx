import { useModal } from "../.."
import { Button, Spinner } from "@gd/ui"
import { rspc } from "@/utils/rspcClient"
import { For, Match, Show, Switch, createSignal } from "solid-js"
import { ImportEntityStatus } from "@gd/core_module/bindings"
import EntityCard from "@/components/Card/EntityCard"
import SingleEntity, { setInstances, setStep } from "./SingleEntity"

import { Trans } from "@gd/i18n"
import { isDownloaded } from "./SingleImport"
import { taskIds } from "@/utils/import"
import { ENTITIES } from "@/utils/constants"

interface Props {
  prevStep: () => void
  isImportInstance?: boolean
}

const [currentEntity, setCurrentEntity] = createSignal<
  ImportEntityStatus | undefined
>()

const ThirdStep = (props: Props) => {
  const modalsContext = useModal()

  const [entity, setEntity] = createSignal<ImportEntityStatus | undefined>()

  const entities = rspc.createQuery(() => ({
    queryKey: ["instance.getImportableEntities"]
  }))

  const handleClickEntity = (ent: ImportEntityStatus) => {
    if (ent.supported) {
      if (currentEntity() && !(currentEntity()?.entity === ent.entity)) {
        setStep("selectionStep")
        setInstances([])
      }
      if (taskIds().every((x) => x === undefined)) {
        setStep("selectionStep")
        setInstances([])
      }
      setEntity(ent)
      setCurrentEntity(ent)
    }
  }

  return (
    <div
      class={`flex flex-col ${
        props.isImportInstance
          ? "h-[600px] w-full"
          : "w-120 lg:w-160 h-full pt-6"
      } box-border`}
    >
      <Switch>
        <Match when={entities.isLoading}>
          <div class="flex h-full w-full items-center justify-center">
            <Spinner class="h-10 w-10" />
          </div>
        </Match>
        <Match when={entity()}>
          <SingleEntity entity={entity()!} setEntity={setEntity} />
        </Match>
        <Match when={!entity()}>
          <div
            class={`flex w-full flex-1 flex-col gap-4 ${
              props.isImportInstance ? "px-4 pt-4" : ""
            }`}
          >
            <Show when={props.isImportInstance}>
              <div class="flex w-full items-center">
                <div class="border-t-1 border-lightSlate-400 flex-1 border-solid" />
                <span class="text-lightSlate-400 flex items-center gap-2 px-3 text-base">
                  <div class="i-hugeicons:rocket-02 text-primary-500 text-sm" />
                  <Trans key="instances:_trn_import_instance" />
                </span>
                <div class="border-t-1 border-lightSlate-400 flex-1 border-solid" />
              </div>
            </Show>
            <ul class="grid grid-cols-3 gap-1.5 p-0">
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
            <div class="flex w-full justify-between">
              <Button
                onClick={() => {
                  props.prevStep()
                }}
                size="large"
                type="secondary"
              >
                <Trans key="onboarding:_trn_prev" />
              </Button>
              <Button
                onClick={() => {
                  modalsContext?.closeModal()
                }}
                size="large"
                type="primary"
              >
                {isDownloaded() ? (
                  <Trans key="onboarding:_trn_done" />
                ) : (
                  <Trans key="onboarding:_trn_skip" />
                )}
              </Button>
            </div>
          </Show>
        </Match>
      </Switch>
    </div>
  )
}

export default ThirdStep
