import { Button } from "@gd/ui"
import { Trans } from "@gd/i18n"

interface Props {
  nextStep: () => void
  duplicatedModsCount: number
}

const IntroStep = (props: Props) => {
  return (
    <div class="box-border flex h-full flex-col justify-between">
      <div class="flex flex-col gap-4">
        <div class="mb-2 flex items-center gap-3">
          <div class="i-hugeicons:alert-01 text-3xl text-yellow-500" />
          <h2 class="m-0 text-xl font-bold">
            <Trans key="content:_trn_duplicates.intro.title" />
          </h2>
        </div>

        <p class="text-lightSlate-700 text-sm leading-6">
          <Trans
            key="content:_trn_duplicates.intro.detected_text"
            options={{ count: props.duplicatedModsCount }}
          />
        </p>

        <p class="text-lightSlate-700 text-sm leading-6">
          <Trans key="content:_trn_duplicates.intro.warning_text" />
        </p>

        <div class="bg-darkSlate-600 mt-2 rounded-lg p-4">
          <h3 class="m-0 mb-2 text-sm font-semibold">
            <Trans key="content:_trn_duplicates.intro.wizard_title" />
          </h3>
          <ul class="text-lightSlate-700 m-0 space-y-2 pl-5 text-sm">
            <li>
              <Trans key="content:_trn_duplicates.intro.wizard_step_1" />
            </li>
            <li>
              <Trans key="content:_trn_duplicates.intro.wizard_step_2" />
            </li>
            <li>
              <Trans key="content:_trn_duplicates.intro.wizard_step_3" />
            </li>
          </ul>
        </div>
      </div>

      <div class="mt-6 flex w-full justify-end">
        <Button type="primary" size="large" onClick={() => props.nextStep()}>
          <Trans key="content:_trn_duplicates.intro.button_start" />
        </Button>
      </div>
    </div>
  )
}

export default IntroStep
