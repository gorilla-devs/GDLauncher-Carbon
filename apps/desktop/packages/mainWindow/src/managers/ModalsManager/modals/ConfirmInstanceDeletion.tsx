import { rspc, queryClient } from "@/utils/rspcClient";
import { UngroupedInstance } from "@gd/core_module/bindings";
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
      onMutate: async (
        instanceId
      ): Promise<
        { previusInstancesUngrouped: UngroupedInstance[] } | undefined
      > => {
        await queryClient.cancelQueries({
          queryKey: ["instance.getInstancesUngrouped"],
        });

        const previusInstancesUngrouped: UngroupedInstance[] | undefined =
          queryClient.getQueryData(["instance.getInstancesUngrouped"]);

        queryClient.setQueryData(
          ["account.getActiveUuid", null],
          (old: UngroupedInstance[] | undefined) => {
            const filteredAccounts = old?.filter(
              (account) => account.id !== instanceId
            );

            if (filteredAccounts) return filteredAccounts;
          }
        );

        if (previusInstancesUngrouped) return { previusInstancesUngrouped };
      },
      onError: (
        error,
        _variables,
        context: { previusInstancesUngrouped: UngroupedInstance[] } | undefined
      ) => {
        addNotification(error.message, "error");

        if (context?.previusInstancesUngrouped) {
          queryClient.setQueryData(
            ["instance.getInstancesUngrouped"],
            context.previusInstancesUngrouped
          );
        }
      },
      onSettled: () => {
        queryClient.invalidateQueries({
          queryKey: ["instance.getInstancesUngrouped"],
        });
      },
    }
  );

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <div class="flex flex-col justify-between w-100 h-60">
        <div class="h-h-full">
          <Trans
            key="instance_confirm_deletion.confirmation_text"
            options={{
              instance_name: props.data?.name,
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
