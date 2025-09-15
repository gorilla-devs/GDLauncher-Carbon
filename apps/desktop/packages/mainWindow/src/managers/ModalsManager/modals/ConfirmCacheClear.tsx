import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { Button, toast } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"

const ConfirmCacheClear = (props: ModalProps) => {
  const [t] = useTransContext()
  const modalsContext = useModal()
  // Note: settings.clearCache endpoint is not available in the backend
  // For now, we'll just close the modal and show a placeholder message
  const handleConfirm = async () => {
    toast.success(t("settings:clear_cache_title"), {
      description: "Cache clearing functionality is currently unavailable"
    })
    modalsContext?.closeModal()
  }

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      height="h-60"
      width="w-100"
    >
      <div class="flex flex-col justify-between h-full">
        <div class="h-h-full">
          <Trans key="settings:clear_cache_confirm_message" />
        </div>
        <div class="flex justify-between w-full">
          <Button
            onClick={() => {
              modalsContext?.closeModal()
            }}
          >
            <Trans key="settings:clear_cache_cancel" />
          </Button>
          <Button type="secondary" onClick={handleConfirm} disabled={false}>
            <Trans key="settings:clear_cache_confirm" />
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default ConfirmCacheClear
