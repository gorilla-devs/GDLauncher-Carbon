import { Button } from "@gd/ui"
import { Trans } from "@gd/i18n"

interface Props {
  nextStep: () => void
  duplicatedModsCount: number
}

const IntroStep = (props: Props) => {
  return (
    <div class="flex flex-col justify-between box-border h-full">
      <div class="flex flex-col gap-4">
        <div class="flex items-center gap-3 mb-2">
          <div class="i-ri:alert-line text-yellow-500 text-3xl" />
          <h2 class="text-xl font-bold m-0">
            <Trans key="instance.duplicates.intro.title" />
          </h2>
        </div>

        <p class="text-lightSlate-700 text-sm leading-6">
          <Trans
            key="instance.duplicates.intro.detected_text"
            options={{ count: props.duplicatedModsCount }}
          />
        </p>

        <p class="text-lightSlate-700 text-sm leading-6">
          <Trans key="instance.duplicates.intro.warning_text" />
        </p>

        <div class="bg-darkSlate-600 rounded-lg p-4 mt-2">
          <h3 class="text-sm font-semibold mb-2 m-0">
            <Trans key="instance.duplicates.intro.wizard_title" />
          </h3>
          <ul class="text-lightSlate-700 text-sm space-y-2 m-0 pl-5">
            <li>
              <Trans key="instance.duplicates.intro.wizard_step_1" />
            </li>
            <li>
              <Trans key="instance.duplicates.intro.wizard_step_2" />
            </li>
            <li>
              <Trans key="instance.duplicates.intro.wizard_step_3" />
            </li>
          </ul>
        </div>
      </div>

      <div class="flex w-full justify-end mt-6">
        <Button
          type="primary"
          size="large"
          onClick={() => props.nextStep()}
        >
          <Trans key="instance.duplicates.intro.button_start" />
        </Button>
      </div>
    </div>
  )
}

export default IntroStep
