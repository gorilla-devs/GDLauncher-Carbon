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

  return (
    <ModalLayout noHeader={props.noHeader} title={props.title} noPadding={true}>
      <div class="w-120 flex flex-col p-4">
        <Show when={data().instanceState === "unlock"}>
          <p>{t("instance_unlock_confirmation")}</p>
        </Show>
        <Show when={data().instanceState === "unpair"}>
          <p>{t("instance_unpair_confirmation")}</p>
        </Show>
        <p>{t("instance_confirm_continue")}</p>
        <div class="flex justify-between mt-8">
          <Button
            type="primary"
            onClick={() => {
              modalContext?.closeModal()
            }}
          >
            {t("instance_cancel")}
          </Button>
          <Button
            type="secondary"
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
            {t("instance_confirm")}
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default Confirmation
