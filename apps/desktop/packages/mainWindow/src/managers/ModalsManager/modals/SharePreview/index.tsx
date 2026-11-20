import { DialogFooter, CancelLink } from "../../DialogFooter"
import { ModalProps, useModal } from "../.."
import ModalLayout from "../../ModalLayout"
import SharePreviewContent from "@/components/SharePreviewContent"
import { useGDNavigate } from "@/managers/NavigationManager"

interface Props {
  shareCode: string
}

function SharePreview(props: ModalProps) {
  const data: () => Props = () => props.data
  const modalsContext = useModal()
  const navigator = useGDNavigate()

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <div class="w-140 flex flex-col gap-4">
        <SharePreviewContent
          shareCode={data()?.shareCode || ""}
          onImportSuccess={() => {
            modalsContext?.closeModal()
            navigator.navigate("/library")
          }}
        />

        <DialogFooter>
          <CancelLink onClick={() => modalsContext?.closeModal()} />
        </DialogFooter>
      </div>
    </ModalLayout>
  )
}

export default SharePreview
