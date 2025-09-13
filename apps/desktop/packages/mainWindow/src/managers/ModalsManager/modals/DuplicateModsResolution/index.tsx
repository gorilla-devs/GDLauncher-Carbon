import { Trans, useTransContext } from "@gd/i18n"
import { Button, Checkbox } from "@gd/ui"
import { Mod } from "@gd/core_module/bindings"
import { For, Show, createMemo, createSignal } from "solid-js"
import { queryClient, rspc } from "@/utils/rspcClient"
import { ModalProps } from "@/managers/ModalsManager"
import ModalLayout from "@/managers/ModalsManager/ModalLayout"

const DuplicateModsResolution = (props: ModalProps) => {
    const [t] = useTransContext()
    const [selectedMods, setSelectedMods] = createSignal<Record<string, boolean>>({})

    const instanceId = props.data?.instanceId
    const duplicateMods: Record<string, Mod[]> = props.data?.duplicateMods || {}

    // Create a map of modIds to their duplicates for easier access
    const duplicateModsArray = createMemo(() => {
        return Object.entries(duplicateMods).map(([modId, mods]) => ({
            modId,
            mods
        }))
    })

    const toggleMod = (modId: string, enabled: boolean) => {
        setSelectedMods((prev) => ({
            ...prev,
            [modId]: enabled
        }))
    }

    // Count how many mods are selected for removal
    const selectedCount = createMemo(() => {
        return Object.values(selectedMods()).filter(Boolean).length
    })

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
        try {
            const modsToDelete = Object.entries(selectedMods())
                .filter(([_, selected]) => selected)
                .map(([modId]) => modId)

            for (const modId of modsToDelete) {
                await deleteModMutation.mutate({
                    instance_id: instanceId,
                    mod_id: modId
                })
            }

            const remainingDuplicates = await queryClient.fetchQuery({
                queryKey: ["instance.checkDuplicateAddons", instanceId]
            })

            if (!remainingDuplicates || Object.keys(remainingDuplicates).length === 0) {
                document.querySelector('[aria-label="Close modal"]')?.dispatchEvent(
                    new MouseEvent('click', { bubbles: true })
                )
            } else {
                setSelectedMods({})
            }
        } catch (error) {
            console.error("Error removing duplicate mods:", error)
        }
        const closeButton = document.querySelector('[aria-label="Close modal"]') as HTMLButtonElement
        if (closeButton) {
            closeButton.click()
        }
    }

    const isLoading = () => deleteModMutation.isPending

    return (
        <ModalLayout title={props.title}>
            <div class="flex flex-col gap-4 p-4">
                <p class="text-lightSlate-100">
                    <Trans key="instance.duplicate_mods_description" />
                </p>

                <div class="max-h-96 overflow-y-auto pr-2">
                    <div class="flex flex-col gap-6">
                        <For each={duplicateModsArray()}>
                            {(group) => (
                                <div class="border-b border-darkSlate-500 pb-4">
                                    <h3 class="mb-2 font-semibold text-lightSlate-50">
                                        {group.modId}
                                    </h3>
                                    <div class="flex flex-col gap-2">
                                        <For each={group.mods}>
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
                            )}
                        </For>
                    </div>
                </div>

                <div class="flex justify-between">
                    <span class="text-lightSlate-300">
                        {selectedCount()} <Trans key="instance.mods_selected_for_removal" />
                    </span>
                    <div class="flex gap-2">
                        <Button
                            disabled={selectedCount() === 0}
                            loading={isLoading()}
                            onClick={handleRemoveSelectedMods}
                        >
                            <Trans key="instance.remove_selected" />
                        </Button>
                    </div>
                </div>
            </div>
        </ModalLayout>
    )
}

export default DuplicateModsResolution