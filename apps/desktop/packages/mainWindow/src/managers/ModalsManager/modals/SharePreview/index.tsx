import { Button } from "@gd/ui"
import { ModalProps, useModal } from "../.."
import ModalLayout from "../../ModalLayout"
import { Trans } from "@gd/i18n"
import SharePreviewContent from "@/components/SharePreviewContent"

interface Props {
  shareCode: string
}

function SharePreview(props: ModalProps) {
  const data: () => Props = () => props.data
  const modalsContext = useModal()

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <div class="w-120 flex flex-col gap-4">
        <SharePreviewContent
          shareCode={data()?.shareCode || ""}
          onImportSuccess={() => modalsContext?.closeModal()}
        />

        {/* Cancel button */}
        <div class="flex justify-start">
          <Button type="secondary" onClick={() => modalsContext?.closeModal()}>
            <Trans key="instances:_trn_share_preview.cancel" />
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default SharePreview
