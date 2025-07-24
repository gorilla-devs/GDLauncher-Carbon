import { rspc } from "@/utils/rspcClient"
import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { Button, createNotification } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"

const ConfirmCacheClear = (props: ModalProps) => {
  const [t] = useTransContext()
  const modalsContext = useModal()
  const addNotification = createNotification()

  const clearCacheMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.clearCache"],
    onSuccess: () => {
      addNotification({
        name: t("settings:clear_cache_title"),
        content: t("settings:clear_cache_success"),
        type: "success"
      })
    },
    onError: (error) => {
      addNotification({
        name: t("settings:clear_cache_title"),
        content: t("settings:clear_cache_error"),
        type: "error"
      })
      console.error("Failed to clear cache:", error)
    }
  }))

  const handleConfirm = async () => {
    await clearCacheMutation.mutateAsync(undefined)
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
          <Button
            type="secondary"
            onClick={handleConfirm}
            disabled={clearCacheMutation.isPending}
          >
            <Trans key="settings:clear_cache_confirm" />
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default ConfirmCacheClear
