import { Button } from "@gd/ui"
import { StepsProps } from "."
import { Trans } from "@gd/i18n"

const ManualStep = (props: StepsProps) => {
  return (
    <div class="w-110 h-65">
      <div class="flex h-full w-full flex-col justify-between">
        <div class="h-13 border-primary-500 flex flex-col items-center justify-center border-2 border-dashed py-4">
          <div class="flex flex-col items-center justify-center gap-2">
            <div class="i-hugeicons:folder-open text-lightSlate-700 w-6 text-xl" />
            <p class="text-lightSlate-700 m-0">
              <Trans key="java:_trn_select_java_zip" options={{ version: 8 }} />
            </p>
          </div>
        </div>
        <p class="text-lightSlate-700 text-center">
          <Trans key="java:_trn_select_required_java_text" />
        </p>
        <div class="flex w-full justify-between gap-4">
          <Button
            rounded
            type="secondary"
            size="large"
            style={{ width: "100%", "max-width": "100%" }}
            onClick={() => {
              props.nextStep?.("intro")
            }}
          >
            <Trans key="java:_trn_step_back" />
          </Button>
          <Button
            rounded
            size="large"
            style={{ width: "100%", "max-width": "100%" }}
            onClick={() => {
              props.nextStep?.("automatic")
            }}
          >
            <Trans key="java:_trn_setup" />
          </Button>
        </div>
      </div>
    </div>
  )
}

export default ManualStep
