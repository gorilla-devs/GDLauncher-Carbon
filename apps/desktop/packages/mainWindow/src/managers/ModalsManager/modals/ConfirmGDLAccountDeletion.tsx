import { rspc } from "@/utils/rspcClient"
import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { Button, toast } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { useGlobalStore } from "@/components/GlobalStoreContext"

const ConfirmGDLAccountDeletion = (props: ModalProps) => {
  const [t] = useTransContext()
  const globalStore = useGlobalStore()

  const modalsContext = useModal()

  const requestAccountDeletionMutation = rspc.createMutation(() => ({
    mutationKey: ["account.requestGdlAccountDeletion"]
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
          <Trans key="accounts:_trn_request_account_deletion_description" />
        </div>
        <div class="flex w-full justify-between">
          <Button
            onClick={() => {
              modalsContext?.closeModal()
            }}
          >
            {t("instances:_trn_instance_confirm_deletion.cancel")}
          </Button>
          <Button
            type="secondary"
            onClick={async () => {
              const uuid = globalStore.accounts.data?.find(
                (account) =>
                  account.uuid === globalStore.settings.data?.gdlAccountId
              )?.uuid

              if (!uuid) {
                throw new Error("No active gdl account")
              }

              await requestAccountDeletionMutation.mutateAsync(uuid)
              toast.success("Deletion Request Sent", {
                description: "Check your email"
              })

              modalsContext?.closeModal()
            }}
          >
            {t("instances:_trn_instance_confirm_deletion.delete")}
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default ConfirmGDLAccountDeletion
