import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { Trans } from "@gd/i18n"
import { Button } from "@gd/ui"
import { rspc } from "@/utils/rspcClient"
import { DialogFooter } from "../DialogFooter"
import { TextLink } from "@/components/TextLink"
import { HeroIcon } from "@/components/HeroIcon"

const BetaPrompt = (props: ModalProps) => {
  const modalsContext = useModal()

  const settingsMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setSettings"]
  }))

  const dismissPermanentlyMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.dismissBetaPromptPermanently"]
  }))

  const remindLaterMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.remindBetaPromptLater"]
  }))

  const handleSwitchToBeta = () => {
    settingsMutation.mutate({
      releaseChannel: {
        Set: "beta"
      }
    })
    modalsContext?.closeModal()
  }

  const handleMaybeLater = () => {
    remindLaterMutation.mutate(undefined)
    modalsContext?.closeModal()
  }

  const handleNeverAsk = () => {
    dismissPermanentlyMutation.mutate(undefined)
    modalsContext?.closeModal()
  }

  return (
    <ModalLayout noHeader={props.noHeader} title={props.title}>
      <div class="w-120 flex flex-col gap-8">
        {/* Content */}
        <div class="flex flex-col items-center gap-6">
          {/* Hero Icon */}
          <HeroIcon icon="i-hugeicons:rocket-01" color="primary" glow />

          {/* Title */}
          <h2 class="text-lightSlate-50 text-center text-xl font-bold">
            <Trans key="modals:_trn_beta_prompt_body" />
          </h2>

          {/* Description Box */}
          <div class="bg-darkSlate-700/50 border-darkSlate-600 w-full rounded-lg border p-4">
            <p class="text-lightSlate-400 text-center text-sm leading-relaxed">
              <Trans key="modals:_trn_beta_prompt_description" />
            </p>
          </div>
        </div>

        <DialogFooter>
          <TextLink data-testid="beta-prompt-never" onClick={handleNeverAsk}>
            <Trans key="modals:_trn_beta_prompt_never" />
          </TextLink>
          <TextLink onClick={handleMaybeLater}>
            <Trans key="modals:_trn_beta_prompt_later" />
          </TextLink>
          <Button type="primary" onClick={handleSwitchToBeta} size="large">
            <div class="flex items-center gap-2">
              <i class="i-hugeicons:rocket-01 h-4 w-4" />
              <Trans key="modals:_trn_beta_prompt_switch" />
            </div>
          </Button>
        </DialogFooter>
      </div>
    </ModalLayout>
  )
}

export default BetaPrompt
