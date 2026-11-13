import { rspc } from "@/utils/rspcClient"
import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { Button, Spinner, toast } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import type { NamespacedTranslationKey } from "@gd/i18n"
import { useGDNavigate } from "@/managers/NavigationManager"
import { createMemo, createSignal, For, Match, Show, Switch } from "solid-js"
import { ListGroup, ListServerGroup } from "@gd/core_module/bindings"
import type { FolderKind } from "@/pages/Library/types"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { deletionTarget } from "@/utils/deletionTarget"
import { listedAddonNames } from "@/utils/addonDeletion"
import {
  FolderDeletionDialog,
  FolderDeletionMode,
  folderDeleteAllLabelKey,
  folderDeletionDialog,
  folderKeepLabelKey,
  folderRemoveLabelKey
} from "@/utils/folderDeletion"
import {
  DeletionNoteBox,
  DeletionNoteLine,
  DeletionTargetLine
} from "./DeletionNote"

export interface ConfirmBatchFolderDeletionData {
  kind: FolderKind
  folders: (ListGroup | ListServerGroup)[]
  /**
   * The action a menu entry named for one folder. Without it the dialog
   * asks what happens to the instances inside.
   */
  mode?: FolderDeletionMode
  onComplete?: () => void
}

export async function deleteFolderTargets(
  kind: FolderKind | undefined,
  mode: FolderDeletionMode,
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

/**
 * Asks before a library folder is removed. A folder's menu entry names what
 * happens to the instances in it, and the dialog then offers only that:
 * removing the folder alone, or deleting the instances with it. A selection
 * of folders names nothing, so the dialog offers both, each button saying
 * what it does. Servers are never deleted through their folder.
 */
const ConfirmBatchFolderDeletion = (props: ModalProps) => {
  const [t] = useTransContext()
  const modalsContext = useModal()
  const navigator = useGDNavigate()
  const globalStore = useGlobalStore()

  const [isDeleting, setIsDeleting] = createSignal(false)
  const [deletedCount, setDeletedCount] = createSignal(0)
  const [error, setError] = createSignal("")
  const [operationMode, setOperationMode] = createSignal<FolderDeletionMode>()
  // What the dialog showed when its action first ran. A retry after a
  // partial failure keeps it, whatever was deleted in the meantime.
  const [admittedDialog, setAdmittedDialog] =
    createSignal<FolderDeletionDialog>()

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

  const dialog = createMemo(
    () =>
      admittedDialog() ??
      folderDeletionDialog(
        kind(),
        props?.data?.mode,
        folders().length,
        totalInstanceCount()
      )
  )
  const several = () => folders().length > 1

  // The instances a delete removes, by name.
  const instanceNames = createMemo(() =>
    listedAddonNames(
      (globalStore.instances.data || [])
        .filter((instance) =>
          folders().some((folder) => folder.id === instance.group_id)
        )
        .map((instance) => instance.name)
    )
  )

  const settings = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }))
  const target = () => deletionTarget(settings.data?.deletionThroughRecycleBin)

  const title = () =>
    dialog() === "deleteAll"
      ? t("instances:_trn_delete_folder_all_title")
      : t(folderRemoveLabelKey(folders().length), { count: folders().length })

  const deleteGroupMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.deleteGroup"]
  }))

  const deleteGroupWithInstancesMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.deleteGroupWithInstances"]
  }))
  const deleteServerGroupMutation = rspc.createMutation(() => ({
    mutationKey: ["server.deleteServerGroup"]
  }))

  const handleDelete = async (mode: FolderDeletionMode) => {
    if (isDeleting()) return
    const admittedMode = operationMode()
    if (admittedMode !== undefined && admittedMode !== mode) return
    if (admittedDialog() === undefined) setAdmittedDialog(dialog())
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

  const cancelButton = (fullWidth?: boolean) => (
    <Button
      fullWidth={fullWidth}
      data-testid="folder-deletion-cancel"
      onClick={() => {
        modalsContext?.closeModal()
      }}
    >
      <div class="i-hugeicons:cancel-01" />
      {t("instances:_trn_instance_confirm_deletion.cancel")}
    </Button>
  )

  // The labels are read where they are shown, so they follow the setting
  // and the instance count as those arrive or change.
  const removeButton = (
    labelKey: () => NamespacedTranslationKey,
    fullWidth?: boolean
  ) => (
    <Button
      type="secondary"
      fullWidth={fullWidth}
      data-testid="folder-deletion-remove"
      disabled={operationMode() === "deleteAll"}
      onClick={() => handleDelete("unlink")}
    >
      <div class="i-hugeicons:folder-remove" />
      <Trans key={labelKey()} options={{ count: folders().length }} />
    </Button>
  )

  const deleteAllButton = (
    labelKey: () => NamespacedTranslationKey,
    fullWidth?: boolean
  ) => (
    <Button
      type="secondary"
      fullWidth={fullWidth}
      textColor={target() === "permanent" ? "!text-red-400" : undefined}
      data-testid="folder-deletion-delete-all"
      disabled={operationMode() === "unlink"}
      onClick={() => handleDelete("deleteAll")}
    >
      <div class="i-hugeicons:delete-02" />
      <Trans key={labelKey()} options={{ count: totalInstanceCount() }} />
    </Button>
  )

  const deletedInstancesNote = () => (
    <Show when={target()}>
      <DeletionNoteBox testId="folder-deletion-note">
        <DeletionTargetLine
          target={target()}
          recycleBin={
            <Trans key="instances:_trn_folder_instances_to_recycle_bin" />
          }
          permanent={<Trans key="instances:_trn_folder_instances_permanent" />}
        />
      </DeletionNoteBox>
    </Show>
  )

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={title()}
      height="h-auto"
      width="w-140"
    >
      <div class="flex flex-col gap-4" data-testid="folder-deletion-dialog">
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
          <div class="bg-darkSlate-700 max-h-60 overflow-y-auto rounded-lg p-3">
            <For each={folderInstanceCounts()}>
              {(item) => (
                <div class="text-lightSlate-200 flex items-center justify-between gap-2 py-1">
                  <div class="flex min-w-0 items-center gap-2">
                    <div class="i-hugeicons:folder-01 h-4 w-4 shrink-0" />
                    <span class="truncate">{item.folder.name}</span>
                  </div>
                  <span class="text-lightSlate-500 shrink-0 text-sm">
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
          <Switch>
            <Match when={dialog() === "remove"}>
              <Show when={totalInstanceCount() > 0}>
                <div class="text-lightSlate-50">
                  <Trans
                    key={
                      kind() === "server"
                        ? several()
                          ? "instances:_trn_folders_remove_servers_stay"
                          : "instances:_trn_folder_remove_servers_stay"
                        : several()
                          ? "instances:_trn_folders_remove_instances_stay"
                          : "instances:_trn_folder_remove_instances_stay"
                    }
                  />
                </div>
              </Show>
              <DeletionNoteBox testId="folder-deletion-note">
                <DeletionNoteLine tone="grey" testId="folder-deletion-kept">
                  <Trans
                    key={
                      kind() === "server"
                        ? "instances:_trn_folder_no_server_deleted"
                        : "instances:_trn_folder_no_instance_deleted"
                    }
                  />
                </DeletionNoteLine>
              </DeletionNoteBox>
            </Match>
            <Match when={dialog() === "deleteAll"}>
              <div class="text-lightSlate-50">
                <Trans key="instances:_trn_folder_delete_all_intro" />
              </div>
              <div
                class="bg-darkSlate-700 text-lightSlate-200 rounded-lg px-3 py-2 text-sm"
                data-testid="folder-deletion-instances"
              >
                <For each={instanceNames().listed}>
                  {(name) => <div class="truncate py-0.5">{name}</div>}
                </For>
                <Show when={instanceNames().more > 0}>
                  <div class="text-lightSlate-500 py-0.5">
                    <Trans
                      key="instances:_trn_confirm_addon_deletion_more"
                      options={{ count: instanceNames().more }}
                    />
                  </div>
                </Show>
              </div>
              {deletedInstancesNote()}
            </Match>
            <Match when={dialog() === "choice"}>
              <div class="text-lightSlate-50">
                <Trans key="instances:_trn_folders_choice_question" />
              </div>
              {deletedInstancesNote()}
            </Match>
          </Switch>
          <Show when={error()}>
            <p role="alert" class="text-red-400">
              {error()}
            </p>
          </Show>
          <Switch>
            <Match when={dialog() === "remove"}>
              <div class="flex w-full justify-between gap-2 pt-2">
                {cancelButton()}
                {removeButton(() => folderRemoveLabelKey(folders().length))}
              </div>
            </Match>
            <Match when={dialog() === "deleteAll"}>
              <div class="flex w-full justify-between gap-2 pt-2">
                {cancelButton()}
                {deleteAllButton(() =>
                  target() === "permanent"
                    ? "instances:_trn_server_delete_permanently"
                    : folderDeleteAllLabelKey(
                        folders().length,
                        totalInstanceCount()
                      )
                )}
              </div>
            </Match>
            <Match when={dialog() === "choice"}>
              <div class="flex w-full flex-col gap-2 pt-2">
                {removeButton(() => folderKeepLabelKey(folders().length), true)}
                {deleteAllButton(
                  () =>
                    folderDeleteAllLabelKey(
                      folders().length,
                      totalInstanceCount()
                    ),
                  true
                )}
                {cancelButton(true)}
              </div>
            </Match>
          </Switch>
        </Show>
      </div>
    </ModalLayout>
  )
}

export default ConfirmBatchFolderDeletion
