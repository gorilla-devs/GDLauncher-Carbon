import { rspc, queryClient } from "@/utils/rspcClient"
import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { toast } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { useGDNavigate } from "@/managers/NavigationManager"
import ConfirmActions from "../ConfirmActions"

const ConfirmInstanceDeletion = (props: ModalProps) => {
  const [t] = useTransContext()
  const modalsContext = useModal()
  const navigator = useGDNavigate()

  const isServer = () => !!props.data?.isServer

  const deleteInstanceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.deleteInstance"],
    onSuccess: async () => {
      // Cancel any ongoing queries for this instance to prevent errors
      await queryClient.cancelQueries({
        queryKey: ["instance.getInstanceDetails", props?.data?.id]
      })
      await queryClient.cancelQueries({
        queryKey: ["instance.getModpackInfo", props?.data?.id]
      })
      await queryClient.cancelQueries({
        queryKey: ["instance.getInstanceMods", props?.data?.id]
      })

      navigator.navigate("/library")
    },
    onError: (error) => {
      toast.error(t("notifications:_trn_cannot_delete_instance"), {
        description: error.message
      })
    }
  }))

  const deleteServerMutation = rspc.createMutation(() => ({
    mutationKey: ["server.deleteServer"],
    onSuccess: async () => {
      await queryClient.cancelQueries({
        queryKey: ["server.getServerDetails", props?.data?.id]
      })
      navigator.navigate("/library?mode=servers")
    },
    onError: (error) => {
      toast.error(t("notifications:_trn_cannot_delete_instance"), {
        description: error.message
      })
    }
  }))

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      height="h-auto"
      width="w-100"
      compact
    >
      <div class="flex flex-col gap-6">
        <div>
          <Trans
            key="instances:_trn_instance_confirm_deletion.confirmation_text"
            options={{ instance_name: props.data?.name }}
          >
            {""}
            <span class="font-bold" />
            {""}
          </Trans>
        </div>
        <ConfirmActions
          busy={false}
          confirmTestId="confirm-instance-deletion"
          onCancel={() => modalsContext?.closeModal()}
          onConfirm={() => {
            modalsContext?.closeModal()
            if (isServer()) {
              deleteServerMutation.mutate(props?.data?.id)
            } else {
              deleteInstanceMutation.mutate(props?.data?.id)
            }
          }}
        >
          <div class="i-hugeicons:delete-02" />
          {t("instances:_trn_instance_confirm_deletion.delete")}
        </ConfirmActions>
      </div>
    </ModalLayout>
  )
}

export default ConfirmInstanceDeletion
