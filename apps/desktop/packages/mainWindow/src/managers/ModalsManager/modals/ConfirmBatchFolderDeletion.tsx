import { rspc } from "@/utils/rspcClient"
import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { Button, Spinner, toast } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { useGDNavigate } from "@/managers/NavigationManager"
import { createMemo, createSignal, For, Show } from "solid-js"
import { ListGroup, ListServerGroup } from "@gd/core_module/bindings"
import type { FolderKind } from "@/pages/Library/types"
import { useGlobalStore } from "@/components/GlobalStoreContext"

export async function deleteFolderTargets(
  kind: FolderKind | undefined,
  mode: "unlink" | "deleteAll",
  folders: readonly { id: number }[],
  actions: {
    unlinkInstance: (id: number) => Promise<unknown>
    deleteInstanceContents: (id: number) => Promise<unknown>
    unlinkServer: (id: number) => Promise<unknown>
  },
  progress: (completed: number) => void = () => {},
  acknowledged = new Set<number>()
) {
  if (kind !== "instance" && kind !== "server")
    throw new Error("Folder kind is required")
  if (kind === "server" && mode === "deleteAll")
    throw new Error("Server folder content deletion is not supported")
  const remove =
    kind === "server"
      ? actions.unlinkServer
      : mode === "unlink"
        ? actions.unlinkInstance
        : actions.deleteInstanceContents
  let completed = acknowledged.size
  const failedIds: number[] = []
  for (const folder of folders) {
    if (acknowledged.has(folder.id)) continue
    try {
      await remove(folder.id)
      acknowledged.add(folder.id)
      progress(++completed)
    } catch {
      failedIds.push(folder.id)
    }
  }
  return { completed, failedIds }
}

const ConfirmBatchFolderDeletion = (props: ModalProps) => {
  const [t] = useTransContext()
  const modalsContext = useModal()
  const navigator = useGDNavigate()
  const globalStore = useGlobalStore()

  const [isDeleting, setIsDeleting] = createSignal(false)
  const [deletedCount, setDeletedCount] = createSignal(0)
  const [error, setError] = createSignal("")
  const [operationMode, setOperationMode] = createSignal<
    "unlink" | "deleteAll"
  >()

  const kind = () => props?.data?.kind as FolderKind | undefined
  const folders = () =>
    (props?.data?.folders || []) as (ListGroup | ListServerGroup)[]
  const onComplete = () => props?.data?.onComplete?.()
  const targetKind = kind()
  const targets = folders().map((folder) => ({ id: folder.id }))
  const acknowledged = new Set<number>()

  // Get instance count for each folder
  const folderInstanceCounts = createMemo(() => {
    return folders().map((folder) => ({
      folder,
      count:
        kind() === "server"
          ? (globalStore.servers.data || []).filter(
              (server) => server.groupId === folder.id
            ).length
          : kind() === "instance"
            ? (globalStore.instances.data || []).filter(
                (instance) => instance.group_id === folder.id
              ).length
            : 0
    }))
  })

  const totalInstanceCount = createMemo(() =>
    folderInstanceCounts().reduce((sum, item) => sum + item.count, 0)
  )

  const deleteGroupMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.deleteGroup"]
  }))

  const deleteGroupWithInstancesMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.deleteGroupWithInstances"]
  }))
  const deleteServerGroupMutation = rspc.createMutation(() => ({
    mutationKey: ["server.deleteServerGroup"]
  }))

  const handleDelete = async (mode: "unlink" | "deleteAll") => {
    if (isDeleting()) return
    const admittedMode = operationMode()
    if (admittedMode !== undefined && admittedMode !== mode) return
    // Acknowledged targets certify only this modal's first supported operation.
    if (
      targetKind === "instance" ||
      (targetKind === "server" && mode === "unlink")
    ) {
      setOperationMode(mode)
    }
    setIsDeleting(true)
    setDeletedCount(acknowledged.size)
    setError("")
    try {
      const result = await deleteFolderTargets(
        targetKind,
        mode,
        targets,
        {
          unlinkInstance: (id) => deleteGroupMutation.mutateAsync(id),
          deleteInstanceContents: (id) =>
            deleteGroupWithInstancesMutation.mutateAsync(id),
          unlinkServer: (id) => deleteServerGroupMutation.mutateAsync(id)
        },
        setDeletedCount,
        acknowledged
      )
      if (result.failedIds.length > 0) {
        const message = t("instances:_trn_folder_deletion_partial", {
          completed: result.completed,
          total: targets.length
        })
        setError(message)
        toast.error(message)
        return
      }
      modalsContext?.closeModal()
      onComplete()
      navigator.navigate(
        targetKind === "server" ? "/library?mode=servers" : "/library"
      )
    } catch (failure) {
      const message =
        failure instanceof Error ? failure.message : String(failure)
      setError(message)
      toast.error(message)
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
                  key="instances:_trn_deleting_folders_progress"
                  options={{
                    current: deletedCount(),
                    total: folders().length
                  }}
                />
              </span>
            </div>
          }
        >
          <div class="text-lightSlate-50">
            <Trans
              key={
                kind() === "server"
                  ? "instances:_trn_batch_server_folder_unlink_confirmation"
                  : "instances:_trn_batch_folder_deletion_confirmation"
              }
              options={{ count: folders().length }}
            />
          </div>
          <div class="bg-darkSlate-700 max-h-60 overflow-y-auto rounded-lg p-3">
            <For each={folderInstanceCounts()}>
              {(item) => (
                <div class="text-lightSlate-200 flex items-center justify-between gap-2 py-1">
                  <div class="flex items-center gap-2">
                    <div class="i-hugeicons:folder-01 h-4 w-4 shrink-0" />
                    <span class="truncate">{item.folder.name}</span>
                  </div>
                  <span class="text-lightSlate-500 text-sm">
                    {item.count}{" "}
                    {kind() === "server"
                      ? item.count === 1
                        ? t("instances:_trn_server_singular")
                        : t("instances:_trn_servers")
                      : item.count === 1
                        ? t("instances:_trn_instance_singular")
                        : t("instances:_trn_instances")}
                  </span>
                </div>
              )}
            </For>
          </div>
          <div class="text-lightSlate-400 text-sm">
            <Trans
              key={
                kind() === "server"
                  ? "instances:_trn_batch_folder_total_servers"
                  : "instances:_trn_batch_folder_total_instances"
              }
              options={{ count: totalInstanceCount() }}
            />
          </div>
          <Show when={error()}>
            <p role="alert" class="text-red-400">
              {error()}
            </p>
          </Show>
          <div class="flex w-full justify-between gap-2 pt-2">
            <Button
              onClick={() => {
                modalsContext?.closeModal()
              }}
            >
              <div class="i-hugeicons:cancel-01" />
              {t("instances:_trn_instance_confirm_deletion.cancel")}
            </Button>
            <div class="flex gap-2">
              <Button
                type="secondary"
                disabled={operationMode() === "deleteAll"}
                onClick={() => handleDelete("unlink")}
              >
                <div class="i-hugeicons:folder-remove" />
                <Trans
                  key="instances:_trn_unlink_folders_button"
                  options={{ count: folders().length }}
                />
              </Button>
              <Show when={kind() === "instance"}>
                <Button
                  type="secondary"
                  disabled={operationMode() === "unlink"}
                  onClick={() => handleDelete("deleteAll")}
                >
                  <div class="i-hugeicons:delete-02" />
                  <Trans
                    key="instances:_trn_delete_folders_with_instances_button"
                    options={{ count: folders().length }}
                  />
                </Button>
              </Show>
            </div>
          </div>
        </Show>
      </div>
    </ModalLayout>
  )
}

export default ConfirmBatchFolderDeletion
