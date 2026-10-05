import { rspc, queryClient } from "@/utils/rspcClient"
import {
  LOCAL_ERROR_TOAST,
  toastMutationError
} from "@/utils/mutationErrorToast"
import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { Button, Spinner, toast } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { useGDNavigate } from "@/managers/NavigationManager"
import { createSignal, For, Show } from "solid-js"
import { ListServer, ListServerGroup } from "@gd/core_module/bindings"

export async function deleteServerSelection(
  servers: readonly { id: number }[],
  folders: readonly { id: number }[],
  actions: {
    deleteServer: (id: number) => Promise<unknown>
    unlinkFolder: (id: number) => Promise<unknown>
  },
  progress: (completed: number) => void = () => {},
  acknowledged = { servers: new Set<number>(), folders: new Set<number>() }
) {
  let completed = acknowledged.servers.size + acknowledged.folders.size
  const failedIds: number[] = []
  for (const server of servers) {
    if (acknowledged.servers.has(server.id)) continue
    try {
      await actions.deleteServer(server.id)
      acknowledged.servers.add(server.id)
      progress(++completed)
    } catch {
      failedIds.push(server.id)
    }
  }
  // Keep folder ownership intact if any selected member could not be deleted.
  if (failedIds.length === 0) {
    for (const folder of folders) {
      if (acknowledged.folders.has(folder.id)) continue
      try {
        await actions.unlinkFolder(folder.id)
        acknowledged.folders.add(folder.id)
        progress(++completed)
      } catch {
        failedIds.push(folder.id)
      }
    }
  }
  return { completed, failedIds }
}

const ConfirmBatchServerDeletion = (props: ModalProps) => {
  const [t] = useTransContext()
  const modalsContext = useModal()
  const navigator = useGDNavigate()

  const [isDeleting, setIsDeleting] = createSignal(false)
  const [deletedCount, setDeletedCount] = createSignal(0)
  const [error, setError] = createSignal("")

  const servers = () => (props?.data?.servers || []) as ListServer[]
  const folders = () => (props?.data?.folders || []) as ListServerGroup[]
  const onComplete = () => props?.data?.onComplete?.()
  const targets = servers().map((server) => ({ id: server.id }))
  const folderTargets = folders().map((folder) => ({ id: folder.id }))
  const acknowledged = {
    servers: new Set<number>(),
    folders: new Set<number>()
  }

  const deleteServerMutation = rspc.createMutation(() => ({
    mutationKey: ["server.deleteServer"],
    onSuccess: async (_data, serverId) => {
      await queryClient.cancelQueries({
        queryKey: ["server.getServerDetails", serverId]
      })
    },
    meta: LOCAL_ERROR_TOAST,
    onError: (error) => {
      toastMutationError(t("notifications:_trn_cannot_delete_server"), error)
    }
  }))
  const unlinkFolderMutation = rspc.createMutation(() => ({
    mutationKey: ["server.deleteServerGroup"]
  }))

  const handleDelete = async () => {
    if (isDeleting()) return
    setIsDeleting(true)
    setDeletedCount(acknowledged.servers.size + acknowledged.folders.size)
    setError("")
    try {
      const result = await deleteServerSelection(
        targets,
        folderTargets,
        {
          deleteServer: (id) => deleteServerMutation.mutateAsync(id),
          unlinkFolder: (id) => unlinkFolderMutation.mutateAsync(id)
        },
        setDeletedCount,
        acknowledged
      )
      if (result.failedIds.length > 0) {
        const message = t("instances:_trn_folder_deletion_partial", {
          completed: result.completed,
          total: targets.length + folderTargets.length
        })
        setError(message)
        toast.error(message)
        return
      }
      modalsContext?.closeModal()
      onComplete()
      navigator.navigate("/library?mode=servers")
    } finally {
      setIsDeleting(false)
    }
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
                  key={
                    folders().length
                      ? "instances:_trn_deleting_server_selection_progress"
                      : "instances:_trn_deleting_servers_progress"
                  }
                  options={{
                    current: deletedCount(),
                    total: servers().length + folders().length
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
          <Show when={folders().length > 0}>
            <Trans
              key="instances:_trn_batch_server_folder_unlink_confirmation"
              options={{ count: folders().length }}
            />
            <For each={folders()}>
              {(folder) => <div class="text-lightSlate-200">{folder.name}</div>}
            </For>
          </Show>
          <Show when={error()}>
            <p role="alert" class="text-red-400">
              {error()}
            </p>
          </Show>
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
              <Show when={folders().length > 0}>
                {" + "}
                <Trans
                  key="instances:_trn_unlink_folders_button"
                  options={{ count: folders().length }}
                />
              </Show>
            </Button>
          </div>
        </Show>
      </div>
    </ModalLayout>
  )
}

export default ConfirmBatchServerDeletion
