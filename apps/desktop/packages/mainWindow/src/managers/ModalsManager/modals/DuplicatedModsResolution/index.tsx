import ModalLayout from "../../ModalLayout"
import { ModalProps, useModal } from "../.."
import { Steps, toast } from "@gd/ui"
import { Match, Switch, createSignal } from "solid-js"
import IntroStep from "./IntroStep"
import ModSelectionStep, { DuplicatedMod } from "./ModSelectionStep"
import ActionStep, { DuplicateAction } from "./ActionStep"
import SummaryStep from "./SummaryStep"
import { rspc } from "@/utils/rspcClient"
import { useTransContext } from "@gd/i18n"

interface DuplicatedModsData {
  duplicatedMods: DuplicatedMod[]
  instanceId: number
}

const [currentStep, setCurrentStep] = createSignal(0)
const [currentModIndex, setCurrentModIndex] = createSignal(0)
const [selectedVersions, setSelectedVersions] = createSignal<
  Record<string, string>
>({})
const [selectedAction, setSelectedAction] =
  createSignal<DuplicateAction>("disable")

export { currentStep, setCurrentStep }

const DuplicatedModsResolution = (props: ModalProps) => {
  const [t] = useTransContext()
  const modalContext = useModal()
  const data: () => DuplicatedModsData = () => props.data
  const duplicatedMods = () => data()?.duplicatedMods || []

  // Mutations for mod management
  const deleteModMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.deleteMod"]
  }))
  const disableModMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.disableMod"]
  }))

  const [isApplying, setIsApplying] = createSignal(false)

  // Initialize selected versions with first version of each mod
  if (Object.keys(selectedVersions()).length === 0) {
    const initial: Record<string, string> = {}
    duplicatedMods().forEach((mod) => {
      initial[mod.name] = mod.versions[0]?.id || ""
    })
    setSelectedVersions(initial)
  }

  const totalSteps = 2 + duplicatedMods().length // Intro + Mods + Action + Summary

  const resolutionSteps = [
    {
      label: t("instance.duplicates.steps.introduction"),
      icon: <div>1</div>,
      onClick: () => {
        setCurrentStep(0)
        setCurrentModIndex(0)
      }
    },
    ...duplicatedMods().map((mod, index) => ({
      label:
        mod.name.length > 15 ? mod.name.substring(0, 15) + "..." : mod.name,
      icon: <div>{index + 2}</div>,
      onClick: () => {
        setCurrentStep(1 + index)
        setCurrentModIndex(index)
      }
    })),
    {
      label: t("instance.duplicates.steps.action"),
      icon: <div>{duplicatedMods().length + 2}</div>,
      onClick: () => {
        setCurrentStep(1 + duplicatedMods().length)
      }
    },
    {
      label: t("instance.duplicates.steps.summary"),
      icon: <div>{duplicatedMods().length + 3}</div>,
      onClick: () => {
        setCurrentStep(2 + duplicatedMods().length)
      }
    }
  ]

  const nextStep = () => {
    if (currentStep() < totalSteps + 1) {
      setCurrentStep((prev) => prev + 1)

      // Update mod index if we're in the mod selection phase
      if (currentStep() > 0 && currentStep() <= duplicatedMods().length) {
        setCurrentModIndex((prev) => prev + 1)
      }
    }
  }

  const prevStep = () => {
    if (currentStep() > 0) {
      setCurrentStep((prev) => prev - 1)

      // Update mod index if we're in the mod selection phase
      if (currentStep() > 1 && currentStep() <= duplicatedMods().length + 1) {
        setCurrentModIndex((prev) => prev - 1)
      }
    }
  }

  const handleVersionSelect = (modName: string, versionId: string) => {
    setSelectedVersions({ ...selectedVersions(), [modName]: versionId })
  }

  const handleActionSelect = (action: DuplicateAction) => {
    setSelectedAction(action)
  }

  const handleFinish = async () => {
    setIsApplying(true)

    try {
      const instanceId = data().instanceId

      // Loop through each duplicated mod
      for (const mod of duplicatedMods()) {
        const selectedVersionId = selectedVersions()[mod.name]

        // Get all versions that were NOT selected
        const unselectedVersions = mod.versions.filter(
          (v) => v.id !== selectedVersionId
        )

        // Disable or delete each unselected version
        for (const version of unselectedVersions) {
          if (selectedAction() === "disable") {
            await disableModMutation.mutateAsync({
              instance_id: instanceId,
              mod_id: version.id
            })
          } else {
            await deleteModMutation.mutateAsync({
              instance_id: instanceId,
              mod_id: version.id
            })
          }
        }
      }

      // Success - reset state and close modal
      setCurrentStep(0)
      setCurrentModIndex(0)
      setSelectedVersions({})
      setSelectedAction("disable")
      modalContext?.closeModal()
    } catch (error) {
      console.error("Failed to apply duplicate resolution:", error)
      toast.error(`Failed to apply changes: ${error}`, {
        duration: 4000
      })
    } finally {
      setIsApplying(false)
    }
  }

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      noPadding={true}
    >
      <div class="select-none box-border flex flex-col p-6 w-200 h-[600px]">
        <div class="w-full mb-8 h-20 flex items-center justify-center overflow-x-auto overflow-y-hidden">
          <div class="w-[90%]">
            <Steps steps={resolutionSteps} currentStep={currentStep()} />
          </div>
        </div>
        <div class="flex-1 overflow-y-auto overflow-x-hidden">
          <Switch>
            <Match when={currentStep() === 0}>
              <IntroStep
                nextStep={nextStep}
                duplicatedModsCount={duplicatedMods().length}
              />
            </Match>

            {duplicatedMods().map((mod, index) => (
              <Match when={currentStep() === index + 1}>
                <ModSelectionStep
                  mod={mod}
                  currentModIndex={index}
                  totalMods={duplicatedMods().length}
                  nextStep={nextStep}
                  prevStep={prevStep}
                  onVersionSelect={(versionId) =>
                    handleVersionSelect(mod.name, versionId)
                  }
                  selectedVersion={selectedVersions()[mod.name]}
                  instanceId={data().instanceId}
                />
              </Match>
            ))}

            <Match when={currentStep() === duplicatedMods().length + 1}>
              <ActionStep
                nextStep={nextStep}
                prevStep={prevStep}
                onActionSelect={handleActionSelect}
                selectedAction={selectedAction()}
              />
            </Match>

            <Match when={currentStep() === duplicatedMods().length + 2}>
              <SummaryStep
                mods={duplicatedMods()}
                selectedVersions={selectedVersions()}
                action={selectedAction()}
                prevStep={prevStep}
                onFinish={handleFinish}
                isApplying={isApplying()}
              />
            </Match>
          </Switch>
        </div>
      </div>
    </ModalLayout>
  )
}

export default DuplicatedModsResolution
