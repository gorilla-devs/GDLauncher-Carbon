import { Trans, useTransContext } from "@gd/i18n"
import { Button, Checkbox, toast } from "@gd/ui"
import ModalLayout from "../ModalLayout"
import { ModalProps, useModal } from ".."
import { rspc } from "@/utils/rspcClient"
import { createSignal } from "solid-js"

const EULA_URL = "https://aka.ms/MinecraftEULA"

function ServerEulaAcceptance(props: ModalProps) {
  const [t] = useTransContext()
  const modalsContext = useModal()
  const [accepted, setAccepted] = createSignal(false)
  const [isSubmitting, setIsSubmitting] = createSignal(false)

  const serverId = () => props.data?.server_id as number

  const acceptEulaMutation = rspc.createMutation(() => ({
    mutationKey: ["server.acceptEula"]
  }))

  const startServerMutation = rspc.createMutation(() => ({
    mutationKey: ["server.startServer"]
  }))

  const handleAccept = async () => {
    if (!accepted() || isSubmitting()) return
    setIsSubmitting(true)
    try {
      await acceptEulaMutation.mutateAsync(serverId())
      startServerMutation.mutate(serverId())
      modalsContext?.closeModal()
    } catch (err) {
      toast.error(t("notifications:_trn_eula_accept_failed"), {
        description: err instanceof Error ? err.message : String(err)
      })
    } finally {
      setIsSubmitting(false)
    }
  }

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title} width="w-130">
      <div class="flex flex-col gap-5">
        <div class="flex items-center gap-2 text-xl font-bold text-yellow-400">
          <div class="i-hugeicons:document-validation h-6 w-6 shrink-0" />
          <Trans key="instances:_trn_server_eula_title" />
        </div>
        <div class="text-lightSlate-300 flex flex-col gap-2 text-sm">
          <Trans key="instances:_trn_server_eula_body" />
          <button
            class="text-primary-400 hover:text-primary-300 cursor-pointer self-start text-sm underline"
            onClick={() => window.openExternalLink(EULA_URL)}
            type="button"
          >
            <Trans key="instances:_trn_server_eula_read_link" />
          </button>
        </div>
        <Checkbox
          checked={accepted()}
          onChange={(v) => setAccepted(v)}
          disabled={isSubmitting()}
        >
          <span class="text-lightSlate-100 text-sm">
            <Trans key="instances:_trn_server_eula_agree" />
          </span>
        </Checkbox>
        <div class="flex w-full justify-between pt-2">
          <Button
            onClick={() => modalsContext?.closeModal()}
            disabled={isSubmitting()}
          >
            <div class="i-hugeicons:cancel-01" />
            {t("instances:_trn_instance_confirm_deletion.cancel")}
          </Button>
          <Button
            type="primary"
            onClick={handleAccept}
            disabled={!accepted() || isSubmitting()}
          >
            <div class="i-hugeicons:play" />
            <Trans key="instances:_trn_server_eula_accept_and_start" />
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default ServerEulaAcceptance
