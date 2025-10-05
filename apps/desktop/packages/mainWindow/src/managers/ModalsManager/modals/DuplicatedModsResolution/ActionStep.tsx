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
    <div class="flex flex-col justify-between h-full">
      <div class="flex flex-col gap-4">
        <div class="flex items-center gap-3 mb-2">
          <div class="i-ri:settings-3-fill text-primary-500 text-3xl" />
          <h2 class="text-xl font-bold m-0">
            <Trans key="instance.duplicates.action.title" />
          </h2>
        </div>

        <p class="text-lightSlate-700 text-sm leading-6">
          <Trans key="instance.duplicates.action.question" />
        </p>

        <div class="flex flex-col gap-3 mt-4">
          <div
            class="border border-darkSlate-500 rounded-lg p-5 transition-all duration-200"
            classList={{
              "bg-darkSlate-600 border-primary-500": selectedAction() === "disable",
              "bg-darkSlate-700 hover:bg-darkSlate-650": selectedAction() !== "disable"
            }}
          >
            <Radio
              value="disable"
              checked={selectedAction() === "disable"}
              onChange={handleSelect}
            >
              <div class="flex flex-col gap-2 w-full">
                <div class="flex items-center gap-2">
                  <div class="i-ri:eye-off-fill text-lg" />
                  <span class="font-semibold">
                    <Trans key="instance.duplicates.action.disable_title" />
                  </span>
                </div>
                <p class="text-sm text-lightSlate-600 m-0 ml-6">
                  <Trans key="instance.duplicates.action.disable_description" />
                </p>
              </div>
            </Radio>
          </div>

          <div
            class="border border-darkSlate-500 rounded-lg p-5 transition-all duration-200"
            classList={{
              "bg-darkSlate-600 border-red-500": selectedAction() === "remove",
              "bg-darkSlate-700 hover:bg-darkSlate-650": selectedAction() !== "remove"
            }}
          >
            <Radio
              value="remove"
              checked={selectedAction() === "remove"}
              onChange={handleSelect}
            >
              <div class="flex flex-col gap-2 w-full">
                <div class="flex items-center gap-2">
                  <div class="i-ri:delete-bin-fill text-lg text-red-500" />
                  <span class="font-semibold">
                    <Trans key="instance.duplicates.action.remove_title" />
                  </span>
                </div>
                <p class="text-sm text-lightSlate-600 m-0 ml-6">
                  <Trans key="instance.duplicates.action.remove_description" />
                </p>
              </div>
            </Radio>
          </div>
        </div>
      </div>

      <div class="flex justify-between mt-6">
        <Button
          type="secondary"
          size="large"
          onClick={() => props.prevStep()}
        >
          <Trans key="instance.duplicates.action.button_back" />
        </Button>
        <Button
          type="primary"
          size="large"
          onClick={() => props.nextStep()}
        >
          <Trans key="instance.duplicates.action.button_continue" />
        </Button>
      </div>
    </div>
  )
}

export default ActionStep
