import { rspc, queryClient } from "@/utils/rspcClient"
import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { Button, toast } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { useGDNavigate } from "@/managers/NavigationManager"

const ConfirmInstanceDeletion = (props: ModalProps) => {
  const [t] = useTransContext()
  const modalsContext = useModal()
  const navigator = useGDNavigate()

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

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      height="h-60"
      width="w-100"
    >
      <div class="flex h-full flex-col justify-between">
        <div class="h-h-full">
          <Trans
            key="instances:_trn_instance_confirm_deletion.confirmation_text"
            options={{ instance_name: props.data?.name }}
          >
            {""}
            <span class="font-bold" />
            {""}
          </Trans>
        </div>
        <div class="flex w-full justify-between">
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
            onClick={() => {
              modalsContext?.closeModal()
              deleteInstanceMutation.mutate(props?.data?.id)
            }}
          >
            <div class="i-hugeicons:delete-02" />
            {t("instances:_trn_instance_confirm_deletion.delete")}
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default ConfirmInstanceDeletion
