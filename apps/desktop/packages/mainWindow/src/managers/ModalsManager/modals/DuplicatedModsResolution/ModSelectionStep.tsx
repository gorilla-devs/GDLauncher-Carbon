import { Button, Radio } from "@gd/ui"
import { For, Show, createSignal, createEffect } from "solid-js"
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

  // Update local selection when switching between mods
  createEffect(() => {
    setSelectedVersionLocal(props.selectedVersion || props.mod.versions[0]?.id)
  })

  const handleSelect = (versionId: string | number | string[] | undefined) => {
    if (typeof versionId === "string") {
      setSelectedVersionLocal(versionId)
      props.onVersionSelect(versionId)
    }
  }

  return (
    <div class="flex h-full flex-col">
      <div class="flex shrink-0 flex-col gap-4">
        <div class="mb-2 flex items-start gap-3">
          <div class="flex items-center gap-3">
            <Show
              when={props.mod.modId && props.mod.platform !== undefined}
              fallback={
                <div class="bg-darkSlate-600 flex h-12 w-12 items-center justify-center rounded-lg">
                  <div class="i-hugeicons:puzzle text-darkSlate-400 text-2xl" />
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
                class="h-12 w-12 rounded-lg"
              />
            </Show>
            <div>
              <h2 class="m-0 text-xl font-bold">{props.mod.name}</h2>
              <p class="text-lightSlate-600 m-0 text-sm">
                <Trans
                  key="content:_trn_duplicates.selection.step_indicator"
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
          <Trans key="content:_trn_duplicates.selection.instruction" />
        </p>
      </div>

      <div class="mt-2 flex-1 overflow-y-auto overflow-x-hidden pr-2">
        <div class="flex flex-col gap-3">
          <For each={props.mod.versions}>
            {(version) => (
              <div
                class="border-darkSlate-500 rounded-lg border p-4 transition-all duration-200"
                classList={{
                  "bg-darkSlate-600 border-primary-500":
                    selectedVersionLocal() === version.id,
                  "bg-darkSlate-700 hover:bg-darkSlate-650":
                    selectedVersionLocal() !== version.id
                }}
              >
                <Radio
                  value={version.id}
                  checked={selectedVersionLocal() === version.id}
                  onChange={handleSelect}
                >
                  <div class="flex w-full flex-col gap-1">
                    <div class="flex items-center justify-between">
                      <span class="text-sm font-semibold">
                        {version.fileName}
                      </span>
                      <span class="text-lightSlate-600 bg-darkSlate-800 rounded px-2 py-1 text-xs">
                        {version.fileSize}
                      </span>
                    </div>
                    <div class="text-lightSlate-600 mt-1 flex gap-4 text-xs">
                      <span>
                        <Trans key="content:_trn_duplicates.selection.version_label" />{" "}
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

      <div class="mt-6 flex shrink-0 justify-between">
        <Button type="secondary" size="large" onClick={() => props.prevStep()}>
          <Trans key="content:_trn_duplicates.selection.button_back" />
        </Button>
        <Button type="primary" size="large" onClick={() => props.nextStep()}>
          {props.currentModIndex + 1 === props.totalMods ? (
            <Trans key="content:_trn_duplicates.selection.button_continue" />
          ) : (
            <Trans key="content:_trn_duplicates.selection.button_next" />
          )}
        </Button>
      </div>
    </div>
  )
}

export default ModSelectionStep
