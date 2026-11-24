import { Trans } from "@gd/i18n"
import { Button } from "@gd/ui"
import ModalLayout from "../ModalLayout"
import { DialogFooter } from "../DialogFooter"
import { TextLink } from "@/components/TextLink"
import { ModalProps, useModal } from ".."

export interface ConfirmChoiceData {
  title: string
  message: string
  confirmLabel: string
  onConfirm: () => void
}

/** Asks before an action: a title, one message, Cancel and the action. */
function ConfirmChoice(props: ModalProps) {
  const modalsContext = useModal()
  const data = () => props.data as ConfirmChoiceData

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={data().title}
      height="h-auto"
      width="w-120"
    >
      <div class="flex flex-col gap-4" data-testid="confirm-choice">
        <p class="text-lightSlate-300 m-0 break-words text-sm">
          {data().message}
        </p>
        <DialogFooter class="pt-2">
          <TextLink
            data-testid="confirm-choice-cancel"
            onClick={() => modalsContext?.closeModal()}
          >
            <Trans key="general:_trn_cancel" />
          </TextLink>
          <Button
            type="primary"
            data-testid="confirm-choice-confirm"
            onClick={() => {
              modalsContext?.closeModal()
              data().onConfirm()
            }}
          >
            {data().confirmLabel}
          </Button>
        </DialogFooter>
      </div>
    </ModalLayout>
  )
}

export default ConfirmChoice
