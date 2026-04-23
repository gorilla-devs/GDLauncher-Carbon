import { rspc, queryClient } from "@/utils/rspcClient"
import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { Button, Spinner, toast } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { useGDNavigate } from "@/managers/NavigationManager"
import { createSignal, For, Show } from "solid-js"
import { ListServer } from "@gd/core_module/bindings"

const ConfirmBatchServerDeletion = (props: ModalProps) => {
  const [t] = useTransContext()
  const modalsContext = useModal()
  const navigator = useGDNavigate()

  const [isDeleting, setIsDeleting] = createSignal(false)
  const [deletedCount, setDeletedCount] = createSignal(0)

  const servers = () => (props?.data?.servers || []) as ListServer[]
  const onComplete = () => props?.data?.onComplete?.()

  const deleteServerMutation = rspc.createMutation(() => ({
    mutationKey: ["server.deleteServer"],
    onSuccess: async (_data, serverId) => {
      await queryClient.cancelQueries({
        queryKey: ["server.getServerDetails", serverId]
      })
    },
    onError: (error) => {
      toast.error(t("notifications:_trn_cannot_delete_server"), {
        description: error.message
      })
    }
  }))

  const handleDelete = async () => {
    setIsDeleting(true)
    setDeletedCount(0)

    const serverList = servers()

    for (let i = 0; i < serverList.length; i++) {
      try {
        await deleteServerMutation.mutateAsync(serverList[i].id)
        setDeletedCount(i + 1)
      } catch {
        // Error is handled by onError callback
      }
    }

    setIsDeleting(false)
    modalsContext?.closeModal()
    onComplete()
    navigator.navigate("/library?mode=servers")
  }

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      height="h-auto"
      width="w-120"
    >
      <div class="flex flex-col gap-4">
        <Show
          when={!isDeleting()}
          fallback={
            <div class="flex flex-col items-center gap-4 py-8">
              <Spinner />
              <span class="text-lightSlate-50">
                <Trans
                  key="instances:_trn_deleting_servers_progress"
                  options={{
                    current: deletedCount(),
                    total: servers().length
                  }}
                />
              </span>
            </div>
          }
        >
          <div class="text-lightSlate-50">
            <Trans
              key="instances:_trn_batch_server_deletion_confirmation"
              options={{ count: servers().length }}
            />
          </div>
          <div class="bg-darkSlate-700 max-h-60 overflow-y-auto rounded-lg p-3">
            <For each={servers()}>
              {(server) => (
                <div class="text-lightSlate-200 flex items-center gap-2 py-1">
                  <div class="i-hugeicons:server-01 h-4 w-4 shrink-0" />
                  <span class="truncate">{server.name}</span>
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
            <Button type="secondary" onClick={handleDelete}>
              <div class="i-hugeicons:delete-02" />
              <Trans
                key="instances:_trn_delete_servers_button"
                options={{ count: servers().length }}
              />
            </Button>
          </div>
        </Show>
      </div>
    </ModalLayout>
  )
}

export default ConfirmBatchServerDeletion
