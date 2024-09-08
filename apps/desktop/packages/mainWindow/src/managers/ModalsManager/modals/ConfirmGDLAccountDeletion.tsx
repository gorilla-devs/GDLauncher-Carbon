import { rspc } from "@/utils/rspcClient";
import { ModalProps, useModal } from "..";
import ModalLayout from "../ModalLayout";
import { Button, createNotification } from "@gd/ui";
import { Trans, useTransContext } from "@gd/i18n";
import { useGlobalStore } from "@/components/GlobalStoreContext";

const ConfirmGDLAccountDeletion = (props: ModalProps) => {
  const [t] = useTransContext();
  const addNotification = createNotification();
  const globalStore = useGlobalStore();

  const modalsContext = useModal();

  const requestAccountDeletionMutation = rspc.createMutation(() => ({
    mutationKey: ["account.requestGdlAccountDeletion"]
  }));

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      height="h-60"
      width="w-100"
    >
      <div class="flex flex-col justify-between h-full">
        <div class="h-h-full">
          <Trans key="settings:request_account_deletion_description" />
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
            onClick={async () => {
              const uuid = globalStore.accounts.data?.find(
                (account) =>
                  account.uuid === globalStore.settings.data?.gdlAccountId
              )?.uuid;

              if (!uuid) {
                throw new Error("No active gdl account");
              }

              await requestAccountDeletionMutation.mutateAsync(uuid);
              addNotification({
                name: "Deletion Request Sent",
                content: "Check your email",
                type: "success"
              });

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

export default ConfirmGDLAccountDeletion;
