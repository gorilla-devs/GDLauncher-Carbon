import { rspc } from "@/utils/rspcClient"
import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { CancelLink, DialogFooter } from "../DialogFooter"
import { Button, Spinner } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { useGDNavigate } from "@/managers/NavigationManager"
import { createMemo, createSignal, For, Show, onMount } from "solid-js"
import { ListGroup } from "@gd/core_module/bindings"
import { useGlobalStore } from "@/components/GlobalStoreContext"

const ConfirmBatchFolderDeletion = (props: ModalProps) => {
  const [t] = useTransContext()
  const modalsContext = useModal()
  const navigator = useGDNavigate()
  const globalStore = useGlobalStore()

  let cancelLink: HTMLButtonElement | undefined
  // Enter or Esc right after opening must leave everything as it was.
  onMount(() => cancelLink?.focus({ preventScroll: true }))

  const [isDeleting, setIsDeleting] = createSignal(false)
  const [deletedCount, setDeletedCount] = createSignal(0)
  const [_deleteMode, setDeleteMode] = createSignal<"unlink" | "deleteAll">(
    "unlink"
  )

  const folders = () => (props?.data?.folders || []) as ListGroup[]
  const onComplete = () => props?.data?.onComplete?.()

  // Get instance count for each folder
  const folderInstanceCounts = createMemo(() => {
    const instances = globalStore.instances.data || []
    return folders().map((folder) => ({
      folder,
      count: instances.filter((i) => i.group_id === folder.id).length
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

  const handleDelete = async (mode: "unlink" | "deleteAll") => {
    setDeleteMode(mode)
    setIsDeleting(true)
    setDeletedCount(0)

    const folderList = folders()
    const mutation =
      mode === "unlink" ? deleteGroupMutation : deleteGroupWithInstancesMutation

    for (let i = 0; i < folderList.length; i++) {
      try {
        await mutation.mutateAsync(folderList[i].id)
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
      compact
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
              key="instances:_trn_batch_folder_deletion_confirmation"
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
                    {item.count === 1
                      ? t("instances:_trn_instance_singular")
                      : t("instances:_trn_instances")}
                  </span>
                </div>
              )}
            </For>
          </div>
          <div class="text-lightSlate-400 text-sm">
            <Trans
              key="instances:_trn_batch_folder_total_instances"
              options={{ count: totalInstanceCount() }}
            />
          </div>
          <DialogFooter class="pt-2">
            <CancelLink
              ref={(el) => (cancelLink = el)}
              onClick={() => modalsContext?.closeModal()}
            />
            <div class="flex flex-wrap-reverse items-center justify-end gap-3">
              <Button
                type="secondary"
                onClick={() => handleDelete("deleteAll")}
              >
                <div class="i-hugeicons:delete-02" />
                <Trans
                  key="instances:_trn_delete_folders_with_instances_button"
                  options={{ count: folders().length }}
                />
              </Button>
              <Button onClick={() => handleDelete("unlink")}>
                <div class="i-hugeicons:folder-remove" />
                <Trans
                  key="instances:_trn_unlink_folders_button"
                  options={{ count: folders().length }}
                />
              </Button>
            </div>
          </DialogFooter>
        </Show>
      </div>
    </ModalLayout>
  )
}

export default ConfirmBatchFolderDeletion
