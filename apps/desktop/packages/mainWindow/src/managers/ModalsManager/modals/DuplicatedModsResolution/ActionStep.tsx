import { Button, Radio } from "@gd/ui"
import { createSignal } from "solid-js"
import { Trans } from "@gd/i18n"

export type DuplicateAction = "remove" | "disable"

interface Props {
  nextStep: () => void
  prevStep: () => void
  onActionSelect: (action: DuplicateAction) => void
  selectedAction?: DuplicateAction
}

const ActionStep = (props: Props) => {
  const [selectedAction, setSelectedAction] = createSignal<DuplicateAction>(
    props.selectedAction || "disable"
  )

  const handleSelect = (action: string | number | string[] | undefined) => {
    if (action === "remove" || action === "disable") {
      setSelectedAction(action)
      props.onActionSelect(action)
    }
  }

  return (
    <div class="flex h-full flex-col justify-between">
      <div class="flex flex-col gap-4">
        <div class="mb-2 flex items-center gap-3">
          <div class="i-hugeicons:settings-01 text-primary-500 text-3xl" />
          <h2 class="m-0 text-xl font-bold">
            <Trans key="content:_trn_duplicates.action.title" />
          </h2>
        </div>

        <p class="text-lightSlate-700 text-sm leading-6">
          <Trans key="content:_trn_duplicates.action.question" />
        </p>

        <div class="mt-4 flex flex-col gap-3">
          <div
            class="border-darkSlate-500 rounded-lg border p-5 transition-all duration-200"
            classList={{
              "bg-darkSlate-600 border-primary-500":
                selectedAction() === "disable",
              "bg-darkSlate-700 hover:bg-darkSlate-650":
                selectedAction() !== "disable"
            }}
          >
            <Radio
              value="disable"
              checked={selectedAction() === "disable"}
              onChange={handleSelect}
            >
              <div class="flex w-full flex-col gap-2">
                <div class="flex items-center gap-2">
                  <div class="i-hugeicons:view-off-slash text-lg" />
                  <span class="font-semibold">
                    <Trans key="content:_trn_duplicates.action.disable_title" />
                  </span>
                </div>
                <p class="text-lightSlate-600 m-0 ml-6 text-sm">
                  <Trans key="content:_trn_duplicates.action.disable_description" />
                </p>
              </div>
            </Radio>
          </div>

          <div
            class="border-darkSlate-500 rounded-lg border p-5 transition-all duration-200"
            classList={{
              "bg-darkSlate-600 border-red-500": selectedAction() === "remove",
              "bg-darkSlate-700 hover:bg-darkSlate-650":
                selectedAction() !== "remove"
            }}
          >
            <Radio
              value="remove"
              checked={selectedAction() === "remove"}
              onChange={handleSelect}
            >
              <div class="flex w-full flex-col gap-2">
                <div class="flex items-center gap-2">
                  <div class="i-hugeicons:delete-02 text-lg text-red-500" />
                  <span class="font-semibold">
                    <Trans key="content:_trn_duplicates.action.remove_title" />
                  </span>
                </div>
                <p class="text-lightSlate-600 m-0 ml-6 text-sm">
                  <Trans key="content:_trn_duplicates.action.remove_description" />
                </p>
              </div>
            </Radio>
          </div>
        </div>
      </div>

      <div class="mt-6 flex justify-between">
        <Button type="secondary" size="large" onClick={() => props.prevStep()}>
          <Trans key="content:_trn_duplicates.action.button_back" />
        </Button>
        <Button type="primary" size="large" onClick={() => props.nextStep()}>
          <Trans key="content:_trn_duplicates.action.button_continue" />
        </Button>
      </div>
    </div>
  )
}

export default ActionStep
