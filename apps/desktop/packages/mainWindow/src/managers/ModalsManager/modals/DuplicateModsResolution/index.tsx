import { Trans, useTransContext } from "@gd/i18n"
import { Button, Checkbox } from "@gd/ui"
import { Mod } from "@gd/core_module/bindings"
import { For, Show, createSignal, createEffect } from "solid-js"
import { queryClient, rspc } from "@/utils/rspcClient"
import { ModalProps, useModal } from "@/managers/ModalsManager"
import ModalLayout from "@/managers/ModalsManager/ModalLayout"

const DuplicateModsResolution = (props: ModalProps) => {
    const [t] = useTransContext()
    const modalsContext = useModal()
    const [selectedMods, setSelectedMods] = createSignal<Record<string, boolean>>({})
    const [currentStepIndex, setCurrentStepIndex] = createSignal(0)

    const instanceId = props.data?.instanceId
    const duplicateMods: Record<string, Mod[]> = props.data?.duplicateMods || {}

    // Create a map of modIds to their duplicates for easier access
    const duplicateModsArray = () => {
        return Object.entries(duplicateMods).map(([modId, mods]) => ({
            modId,
            mods
        }))
    }

    const currentStep = () => {
        const steps = duplicateModsArray()
        return steps[currentStepIndex()] || null
    }

    const isLastStep = () => {
        return currentStepIndex() >= duplicateModsArray().length - 1
    }

    // Check if we're on the first step
    const isFirstStep = () => {
        return currentStepIndex() === 0
    }

    // Reset selections when step changes
    createEffect(() => {
        if (currentStep()) {
            setSelectedMods({})
        }
    })

    const toggleMod = (modId: string, enabled: boolean) => {
        setSelectedMods((prev) => ({
            ...prev,
            [modId]: enabled
        }))
    }

    // Count how many mods are selected for removal in current step
    const selectedCount = () => {
        return Object.values(selectedMods()).filter(Boolean).length
    }

    const deleteModMutation = rspc.createMutation(() => ({
        mutationKey: ["instance.deleteMod"],
        onSuccess: () => {
            // Refresh the duplicate mods data
            queryClient.invalidateQueries({
                queryKey: ["instance.checkDuplicateAddons", instanceId]
            })
        }
    }))

    const handleRemoveSelectedMods = async () => {
        // Get all the selected mods from current step

        const modsToDelete = Object.entries(selectedMods())
            .filter(([_, selected]) => selected)
            .map(([modId]) => modId)

        // Delete each selected mod
        for (const modId of modsToDelete) {
            await deleteModMutation.mutate({
                instance_id: instanceId,
                mod_id: modId
            })
        }

        // Move to next step or close modal if this was the last step
        if (isLastStep()) {
            handleCloseModal()
        } else {
            setCurrentStepIndex(prev => prev + 1)
        }
    }

    const handleSkipStep = () => {
        if (isLastStep()) {
            handleCloseModal()
        } else {
            setCurrentStepIndex(prev => prev + 1)
        }
    }

    const handlePreviousStep = () => {
        if (!isFirstStep()) {
            setCurrentStepIndex(prev => prev - 1)
        }
    }

    const handleCloseModal = () => {
        modalsContext?.closeModal()
    }

    const isLoading = () => deleteModMutation.isPending

    if (!currentStep()) {
        return null
    }

    return (
        <ModalLayout title={props.title}>
            <div class="flex flex-col gap-4 p-4">
                {/* Step indicator */}
                <div class="flex items-center justify-between border-b border-darkSlate-500 pb-4">
                    <div class="flex items-center gap-2">
                        <span class="text-lightSlate-100">
                            <Trans key="instance.step" /> {currentStepIndex() + 1} <Trans key="instance.of" /> {duplicateModsArray().length}
                        </span>
                    </div>
                    <div class="text-sm text-lightSlate-400">
                        <Trans key="instance.resolving_mod_id" />: <span class="font-medium text-lightSlate-200">{currentStep().modId}</span>
                    </div>
                </div>

                <p class="text-lightSlate-100">
                    <Trans key="instance.duplicate_mods_step_description" />
                </p>

                <div class="max-h-96 overflow-y-auto pr-2">
                    <div class="flex flex-col gap-4">
                        <div class="border-b border-darkSlate-500 pb-4">
                            <h3 class="mb-2 font-semibold text-lightSlate-50">
                                {currentStep().modId}
                            </h3>
                            <div class="flex flex-col gap-2">
                                <For each={currentStep().mods}>
                                    {(mod) => (
                                        <div class="flex items-center justify-between gap-4 rounded bg-darkSlate-700 p-2">
                                            <div class="flex flex-1 flex-col">
                                                <div class="flex items-center gap-2">
                                                    <Checkbox
                                                        checked={!!selectedMods()[mod.id]}
                                                        onChange={(checked) => toggleMod(mod.id, checked)}
                                                    />
                                                    <span class="font-medium">{mod.filename}</span>
                                                </div>
                                                <Show when={mod.metadata?.version}>
                                                    <span class="ml-6 text-sm text-lightSlate-400">
                                                        v{mod.metadata?.version}
                                                    </span>
                                                </Show>
                                            </div>
                                        </div>
                                    )}
                                </For>
                            </div>
                        </div>
                    </div>
                </div>

                <div class="flex justify-between">
                    <div class="flex items-center gap-2">
                        <span class="text-lightSlate-300">
                            {selectedCount()} <Trans key="instance.mods_selected_for_removal" />
                        </span>
                    </div>
                    <div class="flex gap-2">
                        <Button
                            variant="secondary"
                            onClick={handleSkipStep}
                            disabled={isLoading()}
                        >
                            <Trans key="general.skip" />
                        </Button>
                        <Button
                            disabled={selectedCount() === 0}
                            loading={isLoading()}
                            onClick={handleRemoveSelectedMods}
                        >
                            {isLastStep() ? (
                                <Trans key="instance.finish" />
                            ) : (
                                <Trans key="instance.remove_and_continue" />
                            )}
                        </Button>
                    </div>
                </div>
            </div>
        </ModalLayout>
    )
}

export default DuplicateModsResolution