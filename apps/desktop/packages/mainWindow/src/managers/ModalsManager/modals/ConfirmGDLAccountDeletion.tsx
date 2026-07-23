import { rspc } from "@/utils/rspcClient"
import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { Button, toast } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { AccountEntry } from "@gd/core_module/bindings"

const ConfirmGDLAccountDeletion = (props: ModalProps) => {
  const [t] = useTransContext()
  const globalStore = useGlobalStore()

  const modalsContext = useModal()

  const requestAccountDeletionMutation = rspc.createMutation(() => ({
    mutationKey: ["account.requestGdlAccountDeletion"]
  }))

  const confirmDeletion = async () => {
    const uuid = globalStore.accounts.data?.find(
      (account: AccountEntry) =>
        account.uuid === globalStore.settings.data?.gdlAccountId
    )?.uuid

    if (!uuid) {
      toast.error("Request Failed", {
        description: "No active GDLauncher account found."
      })
      return
    }

    try {
      const result = await requestAccountDeletionMutation.mutateAsync(uuid)
      if (result === "success") {
        toast.success("Deletion Request Sent", {
          description: "Check your email"
        })
      } else if (result.failed.cooldown !== null) {
        toast.error("Too Many Requests", {
          description: `Please try again in ${result.failed.cooldown} seconds`
        })
      } else {
        toast.error("Request Failed", {
          description:
            result.failed.message ??
            "Unable to process deletion request. Please try again later."
        })
      }
      modalsContext?.closeModal()
    } catch {
      // Leave the modal open so the user can retry instead of silently
      // losing the request to an unhandled rejection.
      toast.error("Request Failed", {
        description: "Unable to process deletion request. Please try again later."
      })
    }
  }

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
            disabled={requestAccountDeletionMutation.isPending}
            onClick={() => {
              modalsContext?.closeModal()
            }}
          >
            {t("instances:_trn_instance_confirm_deletion.cancel")}
          </Button>
          <Button
            type="secondary"
            loading={requestAccountDeletionMutation.isPending}
            onClick={confirmDeletion}
          >
            {t("instances:_trn_instance_confirm_deletion.delete")}
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default ConfirmGDLAccountDeletion
