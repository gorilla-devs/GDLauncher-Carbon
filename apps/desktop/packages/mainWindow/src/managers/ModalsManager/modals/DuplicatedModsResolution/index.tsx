import ModalLayout from "../../ModalLayout"
import { ModalProps, useModal } from "../.."
import { Steps, toast } from "@gd/ui"
import { Match, Switch, createSignal } from "solid-js"
import IntroStep from "./IntroStep"
import ModSelectionStep, { DuplicatedMod } from "./ModSelectionStep"
import ActionStep, { DuplicateAction } from "./ActionStep"
import SummaryStep from "./SummaryStep"
import { duplicateResolutionPlan } from "./resolutionPlan"
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
  const [effects, setEffects] = createSignal<
    Record<
      string,
      { token: string; action: DuplicateAction; acknowledged: boolean }
    >
  >({})
  const artifactKey = (
    instanceId: number,
    version: DuplicatedMod["versions"][number]
  ) => JSON.stringify({ instanceId, version })
  const affectedIds = () =>
    duplicatedMods().flatMap((group) =>
      group.versions
        .filter((version) => effects()[artifactKey(data().instanceId, version)])
        .map((version) => version.id)
    )
  const hasUncertainEffects = () =>
    Object.values(effects()).some((effect) => !effect.acknowledged)

  // Initialize selected versions with first version of each mod
  if (Object.keys(selectedVersions()).length === 0) {
    const initial: Record<string, string> = {}
    duplicatedMods().forEach((mod) => {
      initial[mod.groupKey] = mod.versions[0]?.id || ""
    })
    setSelectedVersions(initial)
  }

  // Intro + Select Mods + Action + Summary (total 4 steps, 0-indexed so max is 3)

  const resolutionSteps = [
    {
      label: t("content:_trn_duplicates.steps.introduction"),
      icon: <div>1</div>,
      onClick: () => {
        if (isApplying()) return
        setCurrentStep(0)
        setCurrentModIndex(0)
      }
    },
    {
      label: t("content:_trn_duplicates.steps.select_mods"),
      icon: <div>2</div>,
      onClick: () => {
        if (isApplying()) return
        setCurrentStep(1)
        // Keep currentModIndex as-is (stay on current mod)
      }
    },
    {
      label: t("content:_trn_duplicates.steps.action"),
      icon: <div>3</div>,
      onClick: () => {
        if (!isApplying()) setCurrentStep(2)
      }
    },
    {
      label: t("content:_trn_duplicates.steps.summary"),
      icon: <div>4</div>,
      onClick: () => {
        if (!isApplying()) setCurrentStep(3)
      }
    }
  ]

  const nextStep = () => {
    if (isApplying()) return
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
    if (isApplying()) return
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

  const handleVersionSelect = (groupKey: string, versionId: string) => {
    if (isApplying()) return
    if (affectedIds().includes(versionId)) return
    setSelectedVersions({ ...selectedVersions(), [groupKey]: versionId })
  }

  const handleActionSelect = (action: DuplicateAction) => {
    if (isApplying()) return
    if (hasUncertainEffects() && selectedAction() !== action) return
    setSelectedAction(action)
  }

  const handleFinish = async () => {
    if (isApplying()) return
    const plan = duplicateResolutionPlan(duplicatedMods(), selectedVersions())
    if (!plan) {
      toast.error(t("content:_trn_duplicates.selection.instruction"))
      return
    }
    const instanceId = data().instanceId
    const action = selectedAction()
    if (
      plan.some((version) => {
        const previous = effects()[artifactKey(instanceId, version)]
        return previous?.action === "remove" && action === "disable"
      })
    ) {
      toast.error(t("content:_trn_duplicates.removed_cannot_disable"))
      return
    }
    if (
      Object.values(selectedVersions()).some((id) => affectedIds().includes(id))
    ) {
      toast.error(t("content:_trn_duplicates.attempted_cannot_keep"))
      return
    }
    setIsApplying(true)

    try {
      for (const version of plan) {
        const key = artifactKey(instanceId, version)
        const previous = effects()[key]
        if (previous?.acknowledged && previous.action === action) continue
        if (previous && !previous.acknowledged && previous.action !== action)
          throw new Error(t("content:_trn_duplicates.retry_before_action"))
        const effect =
          previous?.action === action
            ? previous
            : { action, token: crypto.randomUUID(), acknowledged: false }
        setEffects((prev) => ({ ...prev, [key]: effect }))
        const mutation =
          action === "disable" ? disableModMutation : deleteModMutation
        await mutation.mutateAsync({
          instance_id: instanceId,
          mod_id: version.id,
          operation_token: effect.token
        })
        setEffects((prev) => ({
          ...prev,
          [key]: { ...effect, acknowledged: true }
        }))
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
      preventClose={isApplying()}
    >
      <div class="w-180 box-border flex h-[600px] select-none flex-col p-6">
        <div class="flex h-16 w-full items-center justify-center overflow-hidden pb-6 mb-6">
          <div
            class="w-[75%]"
            inert={isApplying()}
            aria-disabled={isApplying()}
            classList={{ "opacity-50 pointer-events-none": isApplying() }}
          >
            <Steps steps={resolutionSteps} currentStep={currentStep()} />
          </div>
        </div>
        <div class="flex-1 overflow-y-auto overflow-x-hidden">
          <Switch>
            <Match when={currentStep() === 0}>
              <IntroStep
                isApplying={isApplying()}
                nextStep={nextStep}
                duplicatedModsCount={duplicatedMods().length}
              />
            </Match>

            <Match when={currentStep() === 1}>
              <ModSelectionStep
                mod={duplicatedMods()[currentModIndex()]}
                currentModIndex={currentModIndex()}
                totalMods={duplicatedMods().length}
                isApplying={isApplying()}
                nextStep={nextStep}
                prevStep={prevStep}
                onVersionSelect={(versionId) =>
                  handleVersionSelect(
                    duplicatedMods()[currentModIndex()].groupKey,
                    versionId
                  )
                }
                selectedVersion={
                  selectedVersions()[
                    duplicatedMods()[currentModIndex()].groupKey
                  ]
                }
                instanceId={data().instanceId}
                unavailableVersions={affectedIds()}
              />
            </Match>

            <Match when={currentStep() === 2}>
              <ActionStep
                isApplying={isApplying()}
                nextStep={nextStep}
                prevStep={prevStep}
                onActionSelect={handleActionSelect}
                selectedAction={selectedAction()}
                actionLocked={hasUncertainEffects()}
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
