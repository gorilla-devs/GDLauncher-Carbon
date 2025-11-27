import { rspc } from "@/utils/rspcClient"
import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { Button } from "@gd/ui"
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

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      height="h-60"
      width="w-100"
    >
      <div class="flex h-full flex-col justify-between">
        <div class="h-h-full">
          <Trans key="accounts:_trn_remove_ms_account_with_gdl_account_removal_description" />
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
              await deleteAccountMutation.mutateAsync(data().uuid)
              modalsContext?.closeModal()

              if (accountsLength() === 1) {
                navigator.navigate("/")
              }
            }}
          >
            {t("accounts:_trn_confirm_removal")}
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default ConfirmMsWithGDLAccountRemoval
