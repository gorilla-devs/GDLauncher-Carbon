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

const DuplicatedModsResolution = (props: ModalProps) => {
  const [t] = useTransContext()
  const modalContext = useModal()
  const data: () => DuplicatedModsData = () => props.data
  const duplicatedMods = () => data()?.duplicatedMods || []

  // Component-level signals - reset on each mount
  const [currentStep, setCurrentStep] = createSignal(0)
  const [currentModIndex, setCurrentModIndex] = createSignal(0)
  const [selectedVersions, setSelectedVersions] = createSignal<
    Record<string, string>
  >({})
  const [selectedAction, setSelectedAction] =
    createSignal<DuplicateAction>("disable")

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

  // Intro + Select Mods + Action + Summary (total 4 steps, 0-indexed so max is 3)

  const resolutionSteps = [
    {
      label: t("content:_trn_duplicates.steps.introduction"),
      icon: <div>1</div>,
      onClick: () => {
        setCurrentStep(0)
        setCurrentModIndex(0)
      }
    },
    {
      label: t("content:_trn_duplicates.steps.select_mods"),
      icon: <div>2</div>,
      onClick: () => {
        setCurrentStep(1)
        // Keep currentModIndex as-is (stay on current mod)
      }
    },
    {
      label: t("content:_trn_duplicates.steps.action"),
      icon: <div>3</div>,
      onClick: () => setCurrentStep(2)
    },
    {
      label: t("content:_trn_duplicates.steps.summary"),
      icon: <div>4</div>,
      onClick: () => setCurrentStep(3)
    }
  ]

  const nextStep = () => {
    if (currentStep() === 1) {
      // On Select Mods step: cycle through mods first
      if (currentModIndex() < duplicatedMods().length - 1) {
        setCurrentModIndex((prev) => prev + 1)
      } else {
        setCurrentStep(2) // Move to Action step
      }
    } else if (currentStep() < 3) {
      setCurrentStep((prev) => prev + 1)
    }
  }

  const prevStep = () => {
    if (currentStep() === 1) {
      // On Select Mods step: cycle back through mods first
      if (currentModIndex() > 0) {
        setCurrentModIndex((prev) => prev - 1)
      } else {
        setCurrentStep(0) // Back to Intro
      }
    } else if (currentStep() === 2) {
      // Back from Action: go to last mod
      setCurrentStep(1)
      setCurrentModIndex(duplicatedMods().length - 1)
    } else if (currentStep() > 0) {
      setCurrentStep((prev) => prev - 1)
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

      // Success - close modal (signals will reset on remount)
      modalContext?.closeModal()
    } catch (error) {
      console.error("Failed to apply duplicate resolution:", error)
      toast.error(`Failed to apply changes: ${String(error)}`, {
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
      <div class="w-200 box-border flex h-[700px] select-none flex-col p-6">
        <div class="mb-8 flex h-20 w-full items-center justify-center overflow-x-auto overflow-y-hidden">
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

            <Match when={currentStep() === 1}>
              <ModSelectionStep
                mod={duplicatedMods()[currentModIndex()]}
                currentModIndex={currentModIndex()}
                totalMods={duplicatedMods().length}
                nextStep={nextStep}
                prevStep={prevStep}
                onVersionSelect={(versionId) =>
                  handleVersionSelect(
                    duplicatedMods()[currentModIndex()].name,
                    versionId
                  )
                }
                selectedVersion={
                  selectedVersions()[duplicatedMods()[currentModIndex()].name]
                }
                instanceId={data().instanceId}
              />
            </Match>

            <Match when={currentStep() === 2}>
              <ActionStep
                nextStep={nextStep}
                prevStep={prevStep}
                onActionSelect={handleActionSelect}
                selectedAction={selectedAction()}
              />
            </Match>

            <Match when={currentStep() === 3}>
              <SummaryStep
                mods={duplicatedMods()}
                selectedVersions={selectedVersions()}
                action={selectedAction()}
                prevStep={prevStep}
                onFinish={handleFinish}
                instanceId={data()?.instanceId}
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
