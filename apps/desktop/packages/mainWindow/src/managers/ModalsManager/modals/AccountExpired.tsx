import { useGDNavigate } from "@/managers/NavigationManager"
import { Trans } from "@gd/i18n"
import { Button } from "@gd/ui"
import ModalLayout from "../ModalLayout"
import { ModalProps, useModal } from ".."
import { rspc } from "@/utils/rspcClient"
import { useGlobalStore } from "@/components/GlobalStoreContext"

function AccountExpired(props: ModalProps) {
  const navigator = useGDNavigate()
  const modalsContext = useModal()
  const globalStore = useGlobalStore()

  const launchInstanceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.launchInstance"]
  }))

  const email = () => {
    const account = globalStore.currentlySelectedAccount()
    if (!account) return ""

    return account.type.type === "microsoft" ? account.type.value.email : ""
  }

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <div class="w-120 flex min-h-60 flex-col justify-between gap-2 overflow-hidden">
        <div class="flex flex-col gap-8">
          <div class="text-xl font-bold">
            <Trans key="accounts:_trn_account_expired.expiration_text" />
          </div>
          <div>
            {globalStore.currentlySelectedAccount()?.username} - {email()}
          </div>
          <div class="text-sm">
            <Trans key="accounts:_trn_account_expired.expiration_description" />
          </div>
        </div>

        <div class="flex w-full justify-between">
          <Button
            type="secondary"
            size="large"
            onClick={() => {
              launchInstanceMutation.mutate(props.data?.id)
              modalsContext?.closeModal()
            }}
          >
            <div class="flex items-center gap-2">
              <i class="i-hugeicons:play h-4 w-4" />
              <Trans key="accounts:_trn_account_expired.launch_anyway" />
            </div>
          </Button>
          <Button
            type="primary"
            size="large"
            onClick={() => {
              navigator.navigate("/?addMicrosoftAccount=true")
              modalsContext?.closeModal()
            }}
          >
            <div class="flex items-center gap-2">
              <i class="i-hugeicons:microsoft h-4 w-4" />
              <Trans key="accounts:_trn_account_expired.back_to_login" />
            </div>
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default AccountExpired
