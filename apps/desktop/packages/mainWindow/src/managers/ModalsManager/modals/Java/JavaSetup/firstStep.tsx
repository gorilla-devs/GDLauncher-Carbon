import { Trans } from "@gd/i18n"
import { StepsProps } from "."
import JavaLogo from "/assets/images/icons/java-logo.svg"
import { Button } from "@gd/ui"

const FirstStep = (props: StepsProps) => {
  return (
    <div class="w-110 h-75">
      <div class="flex h-full w-full flex-col justify-between">
        <div class="flex flex-col items-center">
          <img src={JavaLogo} class="h-16 w-16" />
          <h3 class="mb-0">
            <Trans key="java:_trn_java_missing" options={{ version: 8 }} />
          </h3>
        </div>
        <p class="text-darkSlate-300 m-0 text-center">
          <Trans key="java:_trn_missing_java_text" />
        </p>
        <div class="flex w-full justify-between gap-4">
          <Button
            rounded
            type="secondary"
            size="large"
            style={{ width: "100%", "max-width": "100%" }}
            onClick={() => {
              props.nextStep?.("manual")
            }}
          >
            <Trans key="java:_trn_manual_setup" />
          </Button>
          <Button
            rounded
            size="large"
            style={{ width: "100%", "max-width": "100%" }}
            onClick={() => {
              props.nextStep?.("automatic")
            }}
          >
            <Trans key="java:_trn_automatic_setup" />
          </Button>
        </div>
      </div>
    </div>
  )
}

export default FirstStep
