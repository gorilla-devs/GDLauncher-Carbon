import { Button, Radio } from "@gd/ui"
import { For, Show, createSignal } from "solid-js"
import { getModImageUrl } from "@/utils/instances"
import { Trans } from "@gd/i18n"

export interface ModVersion {
  id: string
  fileName: string
  version: string
  dateAdded: string
  fileSize: string
}

export interface DuplicatedMod {
  name: string
  modId?: string
  platform?: string | null
  versions: ModVersion[]
}

interface Props {
  mod: DuplicatedMod
  currentModIndex: number
  totalMods: number
  nextStep: () => void
  prevStep: () => void
  onVersionSelect: (versionId: string) => void
  selectedVersion?: string
  instanceId: number
}

const ModSelectionStep = (props: Props) => {
  const [selectedVersionLocal, setSelectedVersionLocal] = createSignal(
    props.selectedVersion || props.mod.versions[0]?.id
  )

  const handleSelect = (versionId: string | number | string[] | undefined) => {
    if (typeof versionId === "string") {
      setSelectedVersionLocal(versionId)
      props.onVersionSelect(versionId)
    }
  }

  return (
    <div class="flex flex-col h-full">
      <div class="flex-shrink-0 flex flex-col gap-4">
        <div class="flex items-start gap-3 mb-2">
          <div class="flex items-center gap-3">
            <Show
              when={props.mod.modId && props.mod.platform !== undefined}
              fallback={
                <div class="w-12 h-12 rounded-lg bg-darkSlate-600 flex items-center justify-center">
                  <div class="i-ri:puzzle-fill text-2xl text-darkSlate-400" />
                </div>
              }
            >
              <img
                src={getModImageUrl(
                  props.instanceId.toString(),
                  props.mod.modId!,
                  props.mod.platform!
                )}
                alt={props.mod.name}
                class="w-12 h-12 rounded-lg"
              />
            </Show>
            <div>
              <h2 class="text-xl font-bold m-0">{props.mod.name}</h2>
              <p class="text-lightSlate-600 text-sm m-0">
                <Trans
                  key="instance.duplicates.selection.step_indicator"
                  options={{
                    current: props.currentModIndex + 1,
                    total: props.totalMods
                  }}
                />
              </p>
            </div>
          </div>
        </div>

        <p class="text-lightSlate-700 text-sm">
          <Trans key="instance.duplicates.selection.instruction" />
        </p>
      </div>

      <div class="flex-1 overflow-y-auto overflow-x-hidden mt-2 pr-2">
        <div class="flex flex-col gap-3">
          <For each={props.mod.versions}>
            {(version) => (
              <div
                class="border border-darkSlate-500 rounded-lg p-4 transition-all duration-200"
                classList={{
                  "bg-darkSlate-600 border-primary-500": selectedVersionLocal() === version.id,
                  "bg-darkSlate-700 hover:bg-darkSlate-650": selectedVersionLocal() !== version.id
                }}
              >
                <Radio
                  value={version.id}
                  checked={selectedVersionLocal() === version.id}
                  onChange={handleSelect}
                >
                  <div class="flex flex-col gap-1 w-full">
                    <div class="flex items-center justify-between">
                      <span class="font-semibold text-sm">{version.fileName}</span>
                      <span class="text-xs text-lightSlate-600 bg-darkSlate-800 px-2 py-1 rounded">
                        {version.fileSize}
                      </span>
                    </div>
                    <div class="flex gap-4 text-xs text-lightSlate-600 mt-1">
                      <span>
                        <Trans key="instance.duplicates.selection.version_label" />{" "}
                        {version.version}
                      </span>
                    </div>
                  </div>
                </Radio>
              </div>
            )}
          </For>
        </div>
      </div>

      <div class="flex-shrink-0 flex justify-between mt-6">
        <Button
          type="secondary"
          size="large"
          onClick={() => props.prevStep()}
        >
          <Trans key="instance.duplicates.selection.button_back" />
        </Button>
        <Button
          type="primary"
          size="large"
          onClick={() => props.nextStep()}
        >
          {props.currentModIndex + 1 === props.totalMods ? (
            <Trans key="instance.duplicates.selection.button_continue" />
          ) : (
            <Trans key="instance.duplicates.selection.button_next" />
          )}
        </Button>
      </div>
    </div>
  )
}

export default ModSelectionStep
