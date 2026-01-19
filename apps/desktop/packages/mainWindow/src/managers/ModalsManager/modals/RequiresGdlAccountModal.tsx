import { useGDNavigate } from "@/managers/NavigationManager"
import { Trans } from "@gd/i18n"
import { Button } from "@gd/ui"
import ModalLayout from "../ModalLayout"
import { ModalProps, useModal } from ".."

interface Data {
  returnPath?: string
}

function RequiresGdlAccountModal(props: ModalProps) {
  const navigator = useGDNavigate()
  const modalsContext = useModal()

  const data = () => props.data as Data | undefined

  const handleLinkAccount = () => {
    const returnTo = data()?.returnPath || "/"
    navigator.navigate(`/?addGdlAccount=true&returnTo=${encodeURIComponent(returnTo)}`)
    modalsContext?.closeModal()
  }

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <div class="w-100 flex min-h-40 flex-col justify-between gap-6 overflow-hidden">
        <div class="flex flex-col gap-4">
          <div class="text-lightSlate-200 text-sm">
            <Trans key="accounts:_trn_requires_gdl_account_modal_description" />
          </div>
        </div>

        <div class="flex w-full justify-between">
          <Button
            type="secondary"
            size="large"
            onClick={() => modalsContext?.closeModal()}
          >
            <Trans key="accounts:_trn_cancel" />
          </Button>
          <Button
            type="primary"
            size="large"
            onClick={handleLinkAccount}
          >
            <Trans key="accounts:_trn_link_account" />
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default RequiresGdlAccountModal
