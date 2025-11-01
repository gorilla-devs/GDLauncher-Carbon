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
              <Trans
                key="java.select_java_zip"
                options={{
                  defaultValue: "Select java {{version}} zip",
                  version: 8
                }}
              />
            </p>
          </div>
        </div>
        <p class="text-lightSlate-700 text-center">
          <Trans
            key="java.select_required_java_text"
            options={{
              defaultValue:
                "Select the required paths to java. Java 8 is used for all the versions < 1.17"
            }}
          />
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
            <Trans key="java.step_back" />
          </Button>
          <Button
            rounded
            size="large"
            style={{ width: "100%", "max-width": "100%" }}
            onClick={() => {
              props.nextStep?.("automatic")
            }}
          >
            <Trans
              key="java.setup"
              options={{
                defaultValue: "Setup"
              }}
            />
          </Button>
        </div>
      </div>
    </div>
  )
}

export default ManualStep
