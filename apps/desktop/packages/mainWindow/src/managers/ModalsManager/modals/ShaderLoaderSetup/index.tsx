import { createSignal, Match, Switch } from "solid-js"
import type {
  ShaderRecommendation,
  ModSource,
  LatestModSource
} from "@gd/core_module/bindings"
import Intro from "./Intro"
import Installing from "./Installing"
import Done from "./Done"
import ModalLayout from "@/managers/ModalsManager/ModalLayout"
import { ModalProps } from "@/managers/ModalsManager"

export interface ShaderLoaderSetupData {
  recommendation: ShaderRecommendation
  instanceId: number
  installLatest: boolean
  modSource?: ModSource
  latestModSource?: LatestModSource
  replacesMod?: string | null
  onComplete?: (taskId: number | null) => void
}

export type WizardStep = "intro" | "installing" | "done"

export interface InstallPlan {
  fileOnly: boolean
}

export interface StepProps {
  data: ShaderLoaderSetupData
  setStep: (step: WizardStep) => void
  setInstallPlan: (plan: InstallPlan) => void
  installPlan: () => InstallPlan
}

const ShaderLoaderSetup = (props: ModalProps) => {
  const [currentStep, setCurrentStep] = createSignal<WizardStep>("intro")
  const [installPlan, setInstallPlan] = createSignal<InstallPlan>({
    fileOnly: false
  })

  const data = (): ShaderLoaderSetupData => props?.data

  return (
    <ModalLayout
      noHeader={currentStep() === "installing"}
      title={props?.title}
      preventClose={currentStep() === "installing"}
    >
      <div>
        <Switch>
          <Match when={currentStep() === "intro"}>
            <Intro
              data={data()}
              setStep={setCurrentStep}
              setInstallPlan={setInstallPlan}
              installPlan={installPlan}
            />
          </Match>
          <Match when={currentStep() === "installing"}>
            <Installing
              data={data()}
              setStep={setCurrentStep}
              setInstallPlan={setInstallPlan}
              installPlan={installPlan}
            />
          </Match>
          <Match when={currentStep() === "done"}>
            <Done
              data={data()}
              setStep={setCurrentStep}
              setInstallPlan={setInstallPlan}
              installPlan={installPlan}
            />
          </Match>
        </Switch>
      </div>
    </ModalLayout>
  )
}

export default ShaderLoaderSetup
