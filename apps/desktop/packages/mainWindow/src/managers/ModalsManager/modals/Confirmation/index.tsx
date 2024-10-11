import { Button } from "@gd/ui";
import { ModalProps, useModal } from "../..";
import ModalLayout from "../../ModalLayout";
import { Show } from "solid-js";
import { useTransContext } from "@gd/i18n";
import { queryClient, rspc } from "@/utils/rspcClient";
import { instanceId } from "@/utils/browser";

// const [instanceState, setInstanceState] = createSignal<"unlock" | "unpair">(
//   "unlock"
// );
// export { instanceState, setInstanceState };
interface Props {
  instanceState: "unlock" | "unpair";
}
const Confirmation = (props: ModalProps) => {
  const data: () => Props = () => props.data;
  const modalContext = useModal();
  const [t] = useTransContext();
  const updateInstanceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.updateInstance"],

    onMutate: (newData) => {
      queryClient.setQueryData(["instance.getInstanceDetails"], newData);
    }
  }));

  return (
    <ModalLayout noHeader={props.noHeader} title={props.title} noPadding={true}>
      <div class="flex flex-col p-4 w-120">
        <Show when={data().instanceState === "unlock"}>
          <p>{t("instance_unlock_confirmation")}</p>
        </Show>
        <Show when={data().instanceState === "unpair"}>
          <p>{t("instance_unpair_confirmation")}</p>
        </Show>
        <p>{t("instance_confirm_continue")}</p>
        <div class="flex justify-between">
          <Button
            type="primary"
            onClick={() => {
              modalContext?.closeModal();
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
                  instance: instanceId() as number
                });
              } else {
                updateInstanceMutation.mutate({
                  modpackLocked: {
                    Set: null
                  },
                  instance: instanceId() as number
                });
              }
              modalContext?.closeModal();
            }}
          >
            {t("instance_confirm")}
          </Button>
        </div>
      </div>
    </ModalLayout>
  );
};

export default Confirmation;
