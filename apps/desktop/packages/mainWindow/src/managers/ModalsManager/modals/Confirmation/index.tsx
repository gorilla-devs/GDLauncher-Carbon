import { Button } from "@gd/ui"
import { ModalProps, useModal } from "../.."
import ModalLayout from "../../ModalLayout"
import { Show } from "solid-js"
import { useTransContext } from "@gd/i18n"
import { rspc } from "@/utils/rspcClient"

interface Props {
  instanceState: "unlock" | "unpair"
  instanceId: number
}
const Confirmation = (props: ModalProps) => {
  const data: () => Props = () => props.data
  const modalContext = useModal()
  const [t] = useTransContext()
  const updateInstanceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.updateInstance"]
  }))
  // Unpairing cancels an update of the pack that waits and has changed
  // nothing: said before it is confirmed, and only then.
  const pendingSetup = rspc.createQuery(() => ({
    queryKey: ["instance.getPendingSetup", data().instanceId],
    enabled: data().instanceState === "unpair"
  }))

  return (
    <ModalLayout noHeader={props.noHeader} title={props.title} noPadding={true}>
      <div class="w-120 flex flex-col p-4">
        <Show when={data().instanceState === "unlock"}>
          <p>{t("general:_trn_instance_unlock_confirmation")}</p>
        </Show>
        <Show when={data().instanceState === "unpair"}>
          <p>{t("general:_trn_instance_unpair_confirmation")}</p>
          <Show
            when={pendingSetup.data?.pending && pendingSetup.data.can_cancel}
          >
            <p data-testid="unpair-cancels-update">
              {t("general:_trn_instance_unpair_cancels_update")}
            </p>
          </Show>
        </Show>
        <p>{t("instances:_trn_instance_confirm_continue")}</p>
        <div class="mt-8 flex justify-between">
          <Button
            type="primary"
            onClick={() => {
              modalContext?.closeModal()
            }}
          >
            <div class="i-hugeicons:cancel-01" />
            {t("instances:_trn_instance_cancel")}
          </Button>
          <Button
            type="secondary"
            data-testid="confirm-unpair-confirm"
            onClick={() => {
              if (data().instanceState === "unlock") {
                updateInstanceMutation.mutate({
                  modpackLocked: {
                    Set: false
                  },
                  instance: data().instanceId
                })
              } else {
                updateInstanceMutation.mutate({
                  modpackLocked: {
                    Set: null
                  },
                  instance: data().instanceId
                })
              }
              modalContext?.closeModal()
            }}
          >
            <div class="i-hugeicons:tick-02" />
            {t("instances:_trn_instance_confirm")}
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default Confirmation
