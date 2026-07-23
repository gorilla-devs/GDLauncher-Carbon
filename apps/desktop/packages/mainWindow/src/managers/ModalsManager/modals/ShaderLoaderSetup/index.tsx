import { createSignal, Match, Switch, onCleanup } from "solid-js"
import type {
  ShaderRecommendation,
  ModSource,
  LatestModSource
} from "@gd/core_module/bindings"
import { Button } from "@gd/ui"
import { Trans } from "@gd/i18n"
import Installing from "./Installing"
import ModalLayout from "@/managers/ModalsManager/ModalLayout"
import { ModalProps, useModal } from "@/managers/ModalsManager"

export interface ShaderLoaderSetupData {
  recommendation: ShaderRecommendation
  instanceId: number
  installLatest: boolean
  modSource?: ModSource
  latestModSource?: LatestModSource
  replacesMod?: string | null
  onComplete?: (taskId: number | null) => void
}

export type WizardStep = "intro" | "installing"

export interface InstallPlan {
  fileOnly: boolean
}

export interface StepProps {
  data: ShaderLoaderSetupData
  setStep: (step: WizardStep) => void
  setInstallPlan: (plan: InstallPlan) => void
  installPlan: () => InstallPlan
  // Signals completion exactly once. Steps call this instead of
  // `data.onComplete` directly so the owner's loading state is always cleared,
  // including when the modal is dismissed via Escape or a backdrop click.
  complete: (taskId: number | null) => void
}

const Intro = (props: StepProps) => {
  const modalsContext = useModal()
  const recommendation = () => props.data.recommendation

  const startAutoSetup = () => {
    props.setInstallPlan({ fileOnly: false })
    props.setStep("installing")
  }

  const continueWithout = () => {
    props.setInstallPlan({ fileOnly: true })
    props.setStep("installing")
  }

  const cancel = () => {
    props.complete(null)
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
      <div class="mt-4 flex w-full items-center justify-between">
        <Button type="secondary" onClick={cancel}>
          <div class="i-hugeicons:cancel-01" />
          <Trans key="content:_trn_shader_loader_cancel" />
        </Button>
        <div class="flex gap-2">
          <Button type="secondary" onClick={continueWithout}>
            <Trans key="content:_trn_shader_loader_continue_anyway" />
          </Button>
          <Button type="primary" onClick={startAutoSetup}>
            <div class="i-hugeicons:magic-wand-01" />
            <Trans key="content:_trn_shader_loader_auto_setup" />
          </Button>
        </div>
      </div>
    </div>
  )
}

const ShaderLoaderSetup = (props: ModalProps) => {
  const [currentStep, setCurrentStep] = createSignal<WizardStep>("intro")
  const [installPlan, setInstallPlan] = createSignal<InstallPlan>({
    fileOnly: false
  })

  const data = (): ShaderLoaderSetupData => props?.data

  // Notify the owner exactly once, whatever closes the modal. A backdrop click
  // or Escape unmounts this component without running the Cancel/Install
  // handlers, so onCleanup is what guarantees the owner's loading state clears.
  let completed = false
  const complete = (taskId: number | null) => {
    if (completed) return
    completed = true
    data().onComplete?.(taskId)
  }
  onCleanup(() => complete(null))

  return (
    <ModalLayout
      noHeader={currentStep() === "installing"}
      title={props?.title}
      preventClose={currentStep() === "installing"}
    >
      <Switch>
        <Match when={currentStep() === "intro"}>
          <Intro
            data={data()}
            complete={complete}
            setStep={setCurrentStep}
            setInstallPlan={setInstallPlan}
            installPlan={installPlan}
          />
        </Match>
        <Match when={currentStep() === "installing"}>
          <Installing
            data={data()}
            complete={complete}
            setStep={setCurrentStep}
            setInstallPlan={setInstallPlan}
            installPlan={installPlan}
          />
        </Match>
      </Switch>
    </ModalLayout>
  )
}

export default ShaderLoaderSetup
