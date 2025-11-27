import { Button } from "@gd/ui"
import { For, Show } from "solid-js"
import { DuplicatedMod } from "./ModSelectionStep"
import { DuplicateAction } from "./ActionStep"
import { Trans } from "@gd/i18n"
import { getModImageUrl } from "@/utils/instances"

interface Props {
  mods: DuplicatedMod[]
  selectedVersions: Record<string, string>
  action: DuplicateAction
  prevStep: () => void
  onFinish: () => void
  isApplying: boolean
  instanceId: number
}

const SummaryStep = (props: Props) => {
  const getSelectedVersion = (mod: DuplicatedMod) => {
    const selectedId = props.selectedVersions[mod.name]
    return mod.versions.find((v) => v.id === selectedId)
  }

  const getUnselectedVersions = (mod: DuplicatedMod) => {
    const selectedId = props.selectedVersions[mod.name]
    return mod.versions.filter((v) => v.id !== selectedId)
  }

  return (
    <div class="flex h-full flex-col">
      {/* Fixed Header - won't scroll */}
      <div class="flex shrink-0 flex-col gap-4">
        <div class="mb-2 flex items-center gap-3">
          <div class="i-hugeicons:checkmark-badge-01 text-3xl text-green-500" />
          <h2 class="m-0 text-xl font-bold">
            <Trans key="content:_trn_duplicates.summary.title" />
          </h2>
        </div>

        <p class="text-lightSlate-700 text-sm leading-6">
          <Trans key="content:_trn_duplicates.summary.description" />
        </p>

        <div class="bg-darkSlate-600 mb-2 rounded-lg p-4">
          <div class="mb-3 flex items-center gap-2">
            <div class="i-hugeicons:information-circle text-primary-500" />
            <h3 class="m-0 text-sm font-semibold">
              <Trans key="content:_trn_duplicates.summary.action_title" />
            </h3>
          </div>
          <p class="text-lightSlate-700 m-0 ml-6 text-sm">
            <Show
              when={props.action === "disable"}
              fallback={
                <Trans key="content:_trn_duplicates.summary.action_remove" />
              }
            >
              <Trans key="content:_trn_duplicates.summary.action_disable" />
            </Show>
          </p>
        </div>
      </div>

      {/* Scrollable Content - only this section scrolls */}
      <div class="mt-2 flex-1 overflow-y-auto overflow-x-hidden pr-2">
        <div class="flex flex-col gap-3">
          <For each={props.mods}>
            {(mod) => (
              <div class="border-darkSlate-500 bg-darkSlate-700 rounded-lg border p-4">
                <div class="mb-3 flex items-start gap-3">
                  <Show
                    when={mod.modId && mod.platform}
                    fallback={
                      <div class="bg-darkSlate-600 flex h-10 w-10 items-center justify-center rounded-lg">
                        <div class="i-hugeicons:puzzle text-darkSlate-400 text-xl" />
                      </div>
                    }
                  >
                    <img
                      src={getModImageUrl(
                        props.instanceId.toString(),
                        mod.modId!,
                        mod.platform!
                      )}
                      alt={mod.name}
                      class="h-10 w-10 rounded-lg"
                    />
                  </Show>
                  <div class="flex-1">
                    <h4 class="m-0 mb-1 text-sm font-semibold">{mod.name}</h4>

                    <Show when={getSelectedVersion(mod)}>
                      {(version) => (
                        <div class="bg-darkSlate-800 mb-2 rounded p-2">
                          <div class="flex items-center gap-2 text-xs">
                            <div class="i-hugeicons:tick-02 text-green-500" />
                            <span class="text-green-400">
                              <Trans key="content:_trn_duplicates.summary.status_keep" />
                            </span>
                            <span class="text-lightSlate-500">
                              {version().fileName}
                            </span>
                          </div>
                        </div>
                      )}
                    </Show>

                    <div class="space-y-1">
                      <For each={getUnselectedVersions(mod)}>
                        {(version) => (
                          <div class="text-lightSlate-600 flex items-center gap-2 text-xs">
                            <div
                              class="text-sm"
                              classList={{
                                "i-hugeicons:view-off-slash text-yellow-500":
                                  props.action === "disable",
                                "i-hugeicons:delete-02 text-red-500":
                                  props.action === "remove"
                              }}
                            />
                            <span
                              classList={{
                                "text-yellow-400": props.action === "disable",
                                "text-red-400": props.action === "remove"
                              }}
                            >
                              <Show
                                when={props.action === "disable"}
                                fallback={
                                  <Trans key="content:_trn_duplicates.summary.status_remove" />
                                }
                              >
                                <Trans key="content:_trn_duplicates.summary.status_disable" />
                              </Show>
                            </span>
                            <span>{version.fileName}</span>
                          </div>
                        )}
                      </For>
                    </div>
                  </div>
                </div>
              </div>
            )}
          </For>
        </div>
      </div>

      {/* Fixed Footer - won't scroll */}
      <div class="mt-6 flex shrink-0 justify-between">
        <Button
          type="secondary"
          size="large"
          onClick={() => props.prevStep()}
          disabled={props.isApplying}
        >
          <Trans key="content:_trn_duplicates.summary.button_back" />
        </Button>
        <Button
          type="primary"
          size="large"
          onClick={() => props.onFinish()}
          loading={props.isApplying}
          disabled={props.isApplying}
        >
          <Trans key="content:_trn_duplicates.summary.button_apply" />
        </Button>
      </div>
    </div>
  )
}

export default SummaryStep
