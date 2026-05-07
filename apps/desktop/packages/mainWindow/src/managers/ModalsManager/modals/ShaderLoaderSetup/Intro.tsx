import { Button } from "@gd/ui"
import { Trans } from "@gd/i18n"
import { Match, Switch } from "solid-js"
import type { StepProps } from "."
import { useModal } from "@/managers/ModalsManager"

const Intro = (props: StepProps) => {
  const modalsContext = useModal()

  const recommendation = () => props.data.recommendation

  const startAutoSetup = () => {
    props.setInstallPlan({ fileOnly: false })
    props.setStep("installing")
  }

  const continueAnyway = () => {
    props.setInstallPlan({ fileOnly: true })
    props.setStep("installing")
  }

  const cancel = () => {
    props.data.onComplete?.(null)
    modalsContext?.closeModal()
  }

  return (
    <div class="w-130 flex flex-col gap-3">
      <div class="flex flex-col gap-2">
        <h3 class="m-0 text-lg">
          <Trans key="content:_trn_shader_loader_setup_title" />
        </h3>
        <p class="text-darkSlate-100 m-0 text-sm">
          <Switch>
            <Match when={recommendation().kind === "RequiresModloader"}>
              <Trans key="content:_trn_shader_loader_body_vanilla" />
            </Match>
            <Match
              when={
                recommendation().kind === "RecommendLoader" &&
                (recommendation() as { recommended: string }).recommended ===
                  "Iris"
              }
            >
              <Trans key="content:_trn_shader_loader_body_iris" />
            </Match>
            <Match
              when={
                recommendation().kind === "RecommendLoader" &&
                (recommendation() as { recommended: string }).recommended ===
                  "Oculus"
              }
            >
              <Trans key="content:_trn_shader_loader_body_oculus" />
            </Match>
          </Switch>
        </p>
      </div>
      <div class="mt-4 flex justify-end gap-2">
        <Button rounded type="secondary" onClick={cancel}>
          <Trans key="content:_trn_shader_loader_cancel" />
        </Button>
        <Button rounded type="secondary" onClick={continueAnyway}>
          <Trans key="content:_trn_shader_loader_continue_anyway" />
        </Button>
        <Button rounded onClick={startAutoSetup}>
          <Trans key="content:_trn_shader_loader_auto_setup" />
        </Button>
      </div>
    </div>
  )
}

export default Intro
