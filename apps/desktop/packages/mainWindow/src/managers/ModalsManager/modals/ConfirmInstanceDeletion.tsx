import { rspc } from "@/utils/rspcClient";
import { ModalProps, useModal } from "..";
import ModalLayout from "../ModalLayout";
import { Button, createNotification } from "@gd/ui";
import { Trans, useTransContext } from "@gd/i18n";

const ConfirmInstanceDeletion = (props: ModalProps) => {
  const [t] = useTransContext();
  const modalsContext = useModal();
  const addNotification = createNotification();

  const deleteInstanceMutation = rspc.createMutation(
    ["instance.deleteInstance"],
    {
      onError: (error) => {
        addNotification(error.message, "error");
      }
    }
  );

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      height="h-60"
      width="w-100"
    >
      <div class="flex flex-col justify-between h-full">
        <div class="h-h-full">
          <Trans
            key="instance_confirm_deletion.confirmation_text"
            options={{
              instance_name: props.data?.name
            }}
          >
            {""}
            <span class="font-bold" />
            {""}
          </Trans>
        </div>
        <div class="flex justify-between w-full">
          <Button
            onClick={() => {
              modalsContext?.closeModal();
            }}
          >
            {t("instance_confirm_deletion.cancel")}
          </Button>
          <Button
            type="secondary"
            onClick={() => {
              deleteInstanceMutation.mutate(props?.data?.id);
              modalsContext?.closeModal();
            }}
          >
            {t("instance_confirm_deletion.delete")}
          </Button>
        </div>
      </div>
    </ModalLayout>
  );
};

export default ConfirmInstanceDeletion;
