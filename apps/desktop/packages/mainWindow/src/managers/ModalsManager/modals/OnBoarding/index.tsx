import ModalLayout from "../../ModalLayout"
import { ModalProps } from "../.."
import { Steps } from "@gd/ui"
import { Match, Switch, createSignal } from "solid-js"
import SecondStep from "./SecondStep"
import ThirdStep from "./ThirdStep"
import FirstStep from "./FirstStep"
import { useTransContext } from "@gd/i18n"
import mcCubes from "/assets/images/icons/mc-cubes.png"

const [currentStep, setCurrentStep] = createSignal(0)
export { currentStep, setCurrentStep }
const OnBoarding = (props: ModalProps) => {
  const [t] = useTransContext()

  const onBoardingSteps = [
    {
      label: t("onboarding:_trn_introduction"),
      icon: <div>1</div>,
      onClick: () => setCurrentStep(0)
    },
    {
      label: t("onboarding:_trn_handle_preferences"),
      icon: <div>2</div>,
      onClick: () => setCurrentStep(1)
    },
    {
      label: t("onboarding:_trn_import_instances"),
      icon: <div>3</div>,
      onClick: () => setCurrentStep(2)
    }
  ]

  const nextStep = () => {
    setCurrentStep((prev) => prev + 1)
  }

  const prevStep = () => {
    setCurrentStep((prev) => prev - 1)
  }

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      background={
        <img
          class="w-190 -top-15 absolute -left-10 z-0 opacity-70"
          src={mcCubes}
        />
      }
    >
      <div class="lg:w-160 box-border flex h-full select-none flex-col">
        <div class="h-15 max-w-90 mx-auto w-full">
          <Steps steps={onBoardingSteps} currentStep={currentStep()} />
        </div>
        <div class="h-full">
          <Switch>
            <Match when={currentStep() === 0}>
              <FirstStep nextStep={nextStep} />
            </Match>
            <Match when={currentStep() === 1}>
              <SecondStep nextStep={nextStep} prevStep={prevStep} />
            </Match>
            <Match when={currentStep() === 2}>
              <ThirdStep prevStep={prevStep} />
            </Match>
          </Switch>
        </div>
      </div>
    </ModalLayout>
  )
}

export default OnBoarding
