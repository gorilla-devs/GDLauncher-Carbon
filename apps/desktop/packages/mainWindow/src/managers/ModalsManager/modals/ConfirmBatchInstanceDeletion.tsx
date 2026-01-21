import { rspc, queryClient } from "@/utils/rspcClient"
import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { Button, Spinner, toast } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { useGDNavigate } from "@/managers/NavigationManager"
import { createSignal, For, Show } from "solid-js"
import { ListInstance } from "@gd/core_module/bindings"

const ConfirmBatchInstanceDeletion = (props: ModalProps) => {
  const [t] = useTransContext()
  const modalsContext = useModal()
  const navigator = useGDNavigate()

  const [isDeleting, setIsDeleting] = createSignal(false)
  const [deletedCount, setDeletedCount] = createSignal(0)

  const instances = () => (props?.data?.instances || []) as ListInstance[]
  const onComplete = () => props?.data?.onComplete?.()

  const deleteInstanceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.deleteInstance"],
    onSuccess: async (_data, instanceId) => {
      // Cancel any ongoing queries for this instance to prevent errors
      await queryClient.cancelQueries({
        queryKey: ["instance.getInstanceDetails", instanceId]
      })
      await queryClient.cancelQueries({
        queryKey: ["instance.getModpackInfo", instanceId]
      })
      await queryClient.cancelQueries({
        queryKey: ["instance.getInstanceMods", instanceId]
      })
    },
    onError: (error) => {
      toast.error(t("notifications:_trn_cannot_delete_instance"), {
        description: error.message
      })
    }
  }))

  const handleDelete = async () => {
    setIsDeleting(true)
    setDeletedCount(0)

    const instanceList = instances()

    for (let i = 0; i < instanceList.length; i++) {
      try {
        await deleteInstanceMutation.mutateAsync(instanceList[i].id)
        setDeletedCount(i + 1)
      } catch {
        // Error is handled by onError callback
      }
    }

    setIsDeleting(false)
    modalsContext?.closeModal()
    onComplete()
    navigator.navigate("/library")
  }

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      height="h-auto"
      width="w-120"
    >
      <div class="flex flex-col gap-4">
        <Show when={!isDeleting()} fallback={
          <div class="flex flex-col items-center gap-4 py-8">
            <Spinner />
            <span class="text-lightSlate-50">
              <Trans
                key="instances:_trn_deleting_instances_progress"
                options={{
                  current: deletedCount(),
                  total: instances().length
                }}
              />
            </span>
          </div>
        }>
          <div class="text-lightSlate-50">
            <Trans
              key="instances:_trn_batch_deletion_confirmation"
              options={{ count: instances().length }}
            />
          </div>
          <div class="bg-darkSlate-700 max-h-60 overflow-y-auto rounded-lg p-3">
            <For each={instances()}>
              {(instance) => (
                <div class="text-lightSlate-200 flex items-center gap-2 py-1">
                  <div class="i-hugeicons:folder-01 h-4 w-4 shrink-0" />
                  <span class="truncate">{instance.name}</span>
                </div>
              )}
            </For>
          </div>
          <div class="flex w-full justify-between pt-2">
            <Button
              onClick={() => {
                modalsContext?.closeModal()
              }}
            >
              <div class="i-hugeicons:cancel-01" />
              {t("instances:_trn_instance_confirm_deletion.cancel")}
            </Button>
            <Button
              type="secondary"
              onClick={handleDelete}
            >
              <div class="i-hugeicons:delete-02" />
              <Trans
                key="instances:_trn_delete_instances_button"
                options={{ count: instances().length }}
              />
            </Button>
          </div>
        </Show>
      </div>
    </ModalLayout>
  )
}

export default ConfirmBatchInstanceDeletion
