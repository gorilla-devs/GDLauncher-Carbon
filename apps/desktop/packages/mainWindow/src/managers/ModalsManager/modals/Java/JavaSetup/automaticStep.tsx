import { Progress } from "@gd/ui"
import JavaLogo from "/assets/images/icons/java-logo.svg"
import { Trans } from "@gd/i18n"

const percentage = 40

const AutomaticStep = () => {
  return (
    <div class="w-110 h-50 flex flex-col items-center justify-around">
      <div class="flex flex-col items-center">
        <img src={JavaLogo} class="h-16 w-16" />
        <h3>
          <Trans key="java:_trn_java_missing" options={{ version: 8 }} />
        </h3>
      </div>
      <Progress value={percentage} />
      <p class="mb-0">
        {`${percentage}%`}
        <Trans key="java:_trn_automatic_download_progress" />
      </p>
    </div>
  )
}

export default AutomaticStep
