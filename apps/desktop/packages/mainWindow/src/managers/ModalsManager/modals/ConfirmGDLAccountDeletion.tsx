import { rspc } from "@/utils/rspcClient"
import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { toast } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { AccountEntry } from "@gd/core_module/bindings"
import ConfirmActions from "../ConfirmActions"

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
        description:
          "Unable to process deletion request. Please try again later."
      })
    }
  }

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
          <Trans key="accounts:_trn_request_account_deletion_description" />
        </div>
        <ConfirmActions
          busy={requestAccountDeletionMutation.isPending}
          onCancel={() => modalsContext?.closeModal()}
          onConfirm={confirmDeletion}
        >
          {t("instances:_trn_instance_confirm_deletion.delete")}
        </ConfirmActions>
      </div>
    </ModalLayout>
  )
}

export default ConfirmGDLAccountDeletion
