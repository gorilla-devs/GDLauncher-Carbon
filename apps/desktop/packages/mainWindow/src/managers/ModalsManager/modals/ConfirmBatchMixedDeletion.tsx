import { rspc, queryClient } from "@/utils/rspcClient"
import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { CancelLink, DialogFooter } from "../DialogFooter"
import { Button, Spinner, toast } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { useGDNavigate } from "@/managers/NavigationManager"
import { createMemo, createSignal, For, Show, onMount } from "solid-js"
import { ListGroup, ListInstance } from "@gd/core_module/bindings"
import { useGlobalStore } from "@/components/GlobalStoreContext"

const ConfirmBatchMixedDeletion = (props: ModalProps) => {
  const [t] = useTransContext()
  const modalsContext = useModal()
  const navigator = useGDNavigate()
  const globalStore = useGlobalStore()

  let cancelLink: HTMLButtonElement | undefined
  // Enter or Esc right after opening must leave everything as it was.
  onMount(() => cancelLink?.focus({ preventScroll: true }))

  const [isDeleting, setIsDeleting] = createSignal(false)
  const [deletedCount, setDeletedCount] = createSignal(0)
  const [totalCount, setTotalCount] = createSignal(0)
  const [currentPhase, setCurrentPhase] = createSignal<"instances" | "folders">(
    "instances"
  )

  const instances = () => (props?.data?.instances || []) as ListInstance[]
  const folders = () => (props?.data?.folders || []) as ListGroup[]
  const onComplete = () => props?.data?.onComplete?.()

  // Get instance count for each folder
  const folderInstanceCounts = createMemo(() => {
    const instancesList = globalStore.instances.data || []
    return folders().map((folder) => ({
      folder,
      count: instancesList.filter((i) => i.group_id === folder.id).length
    }))
  })

  const totalFolderInstanceCount = createMemo(() =>
    folderInstanceCounts().reduce((sum, item) => sum + item.count, 0)
  )

  const deleteInstanceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.deleteInstance"],
    onSuccess: async (_data, instanceId) => {
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

  const deleteGroupMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.deleteGroup"]
  }))

  const deleteGroupWithInstancesMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.deleteGroupWithInstances"]
  }))

  const handleDelete = async (folderMode: "unlink" | "deleteAll") => {
    setIsDeleting(true)
    setDeletedCount(0)
    setCurrentPhase("instances")

    const instanceList = instances()
    const folderList = folders()
    setTotalCount(instanceList.length + folderList.length)

    // First delete instances
    for (let i = 0; i < instanceList.length; i++) {
      try {
        await deleteInstanceMutation.mutateAsync(instanceList[i].id)
        setDeletedCount(i + 1)
      } catch {
        // Error is handled by onError callback
      }
    }

    // Then delete folders
    setCurrentPhase("folders")
    const folderMutation =
      folderMode === "unlink"
        ? deleteGroupMutation
        : deleteGroupWithInstancesMutation

    for (let i = 0; i < folderList.length; i++) {
      try {
        await folderMutation.mutateAsync(folderList[i].id)
        setDeletedCount(instanceList.length + i + 1)
      } catch {
        // Error handled
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
      width="w-140"
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
                  key="instances:_trn_deleting_mixed_progress"
                  options={{
                    current: deletedCount(),
                    total: totalCount(),
                    phase: t(
                      currentPhase() === "instances"
                        ? "instances:_trn_instances"
                        : "instances:_trn_folders"
                    )
                  }}
                />
              </span>
            </div>
          }
        >
          <div class="text-lightSlate-50">
            <Trans
              key="instances:_trn_batch_mixed_deletion_confirmation"
              options={{
                instanceCount: instances().length,
                folderCount: folders().length
              }}
            />
          </div>

          {/* Instances section */}
          <Show when={instances().length > 0}>
            <div class="flex flex-col gap-2">
              <span class="text-lightSlate-100 text-sm font-medium">
                <Trans key="instances:_trn_instances" /> ({instances().length})
              </span>
              <div class="bg-darkSlate-700 max-h-40 overflow-y-auto rounded-lg p-3">
                <For each={instances()}>
                  {(instance) => (
                    <div class="text-lightSlate-200 flex items-center gap-2 py-1">
                      <div class="i-hugeicons:box-01 h-4 w-4 shrink-0" />
                      <span class="truncate">{instance.name}</span>
                    </div>
                  )}
                </For>
              </div>
            </div>
          </Show>

          {/* Folders section */}
          <Show when={folders().length > 0}>
            <div class="flex flex-col gap-2">
              <span class="text-lightSlate-100 text-sm font-medium">
                <Trans key="instances:_trn_folders" /> ({folders().length})
              </span>
              <div class="bg-darkSlate-700 max-h-40 overflow-y-auto rounded-lg p-3">
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
                  options={{ count: totalFolderInstanceCount() }}
                />
              </div>
            </div>
          </Show>

          <div class="bg-darkSlate-600 mt-2 rounded-lg p-3">
            <p class="text-lightSlate-300 text-sm">
              <Trans key="instances:_trn_batch_mixed_folder_options" />
            </p>
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
                <Trans key="instances:_trn_delete_all_with_folder_contents" />
              </Button>
              <Button onClick={() => handleDelete("unlink")}>
                <div class="i-hugeicons:folder-remove" />
                <Trans key="instances:_trn_delete_unlink_folders" />
              </Button>
            </div>
          </DialogFooter>
        </Show>
      </div>
    </ModalLayout>
  )
}

export default ConfirmBatchMixedDeletion
