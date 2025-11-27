import { Trans } from "@gd/i18n"
import { ModalProps } from ".."
import ModalLayout from "../ModalLayout"

const Notification = (props: ModalProps) => {
  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <div class="h-130 w-190 overflow-hidden">
        <Trans key="ui:_trn_notification" />
      </div>
    </ModalLayout>
  )
}

export default Notification
