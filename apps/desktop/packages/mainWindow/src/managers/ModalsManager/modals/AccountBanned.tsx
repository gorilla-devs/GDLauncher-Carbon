import { Trans } from "@gd/i18n"
import { Button } from "@gd/ui"
import ModalLayout from "../ModalLayout"
import { ModalProps, useModal } from ".."
import { rspc } from "@/utils/rspcClient"

function AccountBanned(props: ModalProps) {
  const modalsContext = useModal()

  const unlinkMutation = rspc.createMutation(() => ({
    mutationKey: ["account.unlinkGdlAccount"]
  }))

  const handleClose = async () => {
    try {
      await unlinkMutation.mutateAsync(undefined)
    } catch {
      // Ignore errors - user is already banned
    }
    modalsContext?.closeModal()
  }

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <div class="w-100 flex flex-col items-center gap-6 p-4 text-center">
        <div class="i-ri:forbid-2-fill text-red-500 text-5xl" />
        <div class="flex flex-col gap-2">
          <h2 class="text-xl font-bold">
            <Trans key="accounts:_trn_banned_dialog_title" />
          </h2>
          <p class="text-lightSlate-400">
            <Trans key="accounts:_trn_banned_dialog_message" />
          </p>
        </div>
        <Button type="primary" size="large" onClick={handleClose}>
          <Trans key="common:ok" />
        </Button>
      </div>
    </ModalLayout>
  )
}

export default AccountBanned
