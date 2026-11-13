import { rspc, queryClient } from "@/utils/rspcClient"
import {
  LOCAL_ERROR_TOAST,
  toastMutationError
} from "@/utils/mutationErrorToast"
import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { Button } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { useGDNavigate } from "@/managers/NavigationManager"
import {
  deletionStopsServer,
  serverDeleteLabelKey,
  serverDeletionTarget
} from "@/utils/serverDeletion"
import ServerDeletionNote from "./ServerDeletionNote"

export interface ConfirmServerDeletionData {
  id: number
  name: string
}

/**
 * Confirms deleting one server. It says what the instance dialog cannot:
 * the world goes with the folder, where the folder goes, and that a running
 * server is stopped first.
 */
const ConfirmServerDeletion = (props: ModalProps) => {
  const [t] = useTransContext()
  const modalsContext = useModal()
  const navigator = useGDNavigate()
  const data = () => props.data as ConfirmServerDeletionData

  const settings = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }))
  const servers = rspc.createQuery(() => ({
    queryKey: ["server.getAllServers"]
  }))

  const target = () =>
    serverDeletionTarget(settings.data?.deletionThroughRecycleBin)
  const stopsFirst = () =>
    deletionStopsServer(
      servers.data?.find((server) => server.id === data().id)?.state.status
    )

  const deleteServerMutation = rspc.createMutation(() => ({
    mutationKey: ["server.deleteServer"],
    onSuccess: async () => {
      await queryClient.cancelQueries({
        queryKey: ["server.getServerDetails", data().id]
      })
      navigator.navigate("/library?mode=servers")
    },
    meta: LOCAL_ERROR_TOAST,
    onError: (error) => {
      toastMutationError(t("notifications:_trn_cannot_delete_server"), error)
    }
  }))

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      height="h-auto"
      width="w-120"
    >
      <div class="flex flex-col gap-4" data-testid="server-deletion-dialog">
        <div class="flex items-center gap-2 text-lg font-bold">
          <div class="i-hugeicons:server-stack-01 h-5 w-5 shrink-0" />
          <span class="truncate">{data().name}</span>
        </div>
        <div class="text-lightSlate-300 text-sm">
          <Trans key="instances:_trn_server_delete_intro" />
          <ul class="m-0 mt-2 list-disc pl-5">
            <li>
              <Trans key="instances:_trn_server_delete_item_world" />
            </li>
            <li>
              <Trans key="instances:_trn_server_delete_item_files" />
            </li>
          </ul>
        </div>
        <ServerDeletionNote target={target()} stopsFirst={stopsFirst()} />
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
            data-testid="confirm-server-deletion"
            type="secondary"
            textColor={target() === "permanent" ? "!text-red-400" : undefined}
            onClick={() => {
              modalsContext?.closeModal()
              deleteServerMutation.mutate(data().id)
            }}
          >
            <div class="i-hugeicons:delete-02" />
            <Trans key={serverDeleteLabelKey(stopsFirst(), target())} />
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default ConfirmServerDeletion
