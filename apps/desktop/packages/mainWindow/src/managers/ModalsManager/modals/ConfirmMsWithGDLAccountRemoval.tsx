import { rspc } from "@/utils/rspcClient";
import { ModalProps, useModal } from "..";
import ModalLayout from "../ModalLayout";
import { Button } from "@gd/ui";
import { Trans, useTransContext } from "@gd/i18n";

type Props = {
  uuid: string;
};

const ConfirmMsWithGDLAccountRemoval = (props: ModalProps) => {
  const [t] = useTransContext();
  const data: () => Props = () => props?.data;

  const modalsContext = useModal();

  const deleteAccountMutation = rspc.createMutation(() => ({
    mutationKey: ["account.deleteAccount"]
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
          <Trans key="settings:remove_ms_account_with_gdl_account_removal_description" />
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
              await deleteAccountMutation.mutateAsync(data().uuid);
              modalsContext?.closeModal();
            }}
          >
            {t("settings:confirm_removal")}
          </Button>
        </div>
      </div>
    </ModalLayout>
  );
};

export default ConfirmMsWithGDLAccountRemoval;
