import { rspc } from "@/utils/rspcClient"
import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { Button, toast } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
import { useGlobalStore } from "@/components/GlobalStoreContext"
import { useGDNavigate } from "@/managers/NavigationManager"

interface Props {
  uuid: string
}

const ConfirmMsWithGDLAccountRemoval = (props: ModalProps) => {
  const [t] = useTransContext()
  const navigator = useGDNavigate()
  const data: () => Props = () => props?.data

  const modalsContext = useModal()

  const globalStore = useGlobalStore()
  const accountsLength = () => globalStore.accounts.data?.length

  const deleteAccountMutation = rspc.createMutation(() => ({
    mutationKey: ["account.deleteAccount"]
  }))

  const confirmRemoval = async () => {
    try {
      await deleteAccountMutation.mutateAsync(data().uuid)
      modalsContext?.closeModal()

      if (accountsLength() === 1) {
        navigator.navigate("/")
      }
    } catch {
      // Leave the modal open so the user can retry instead of silently
      // losing the request to an unhandled rejection.
      toast.error("Request Failed", {
        description: "Unable to remove the account. Please try again later."
      })
    }
  }

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      height="h-auto"
      width="w-100"
    >
      <div class="flex flex-col gap-6">
        <div>
          <Trans key="accounts:_trn_remove_ms_account_with_gdl_account_removal_description" />
        </div>
        <div class="flex w-full justify-between">
          <Button
            disabled={deleteAccountMutation.isPending}
            onClick={() => {
              modalsContext?.closeModal()
            }}
          >
            {t("instances:_trn_instance_confirm_deletion.cancel")}
          </Button>
          <Button
            type="secondary"
            loading={deleteAccountMutation.isPending}
            onClick={confirmRemoval}
          >
            {t("accounts:_trn_confirm_removal")}
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default ConfirmMsWithGDLAccountRemoval
