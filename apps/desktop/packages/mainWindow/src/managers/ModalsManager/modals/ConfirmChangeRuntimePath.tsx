import { ModalProps, useModal } from "..";
import ModalLayout from "../ModalLayout";
import { Button } from "@gd/ui";
import { Trans, useTransContext } from "@gd/i18n";
import { createResource } from "solid-js";

const ConfirmChangeRuntimePath = (props: ModalProps) => {
  const [t] = useTransContext();
  const modalsContext = useModal();

  const [currentRuntimePath] = createResource(() => {
    return window.getRuntimePath();
  });

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props.title}
      height="h-120"
      width="w-180"
    >
      <div class="flex flex-col justify-between h-full">
        <div class="h-h-full">
          <Trans key="settings:confirm_change_runtime_path_text" />
        </div>
        <div class="h-h-full">
          <div class="text-red-400 font-bold">
            <Trans key="settings:runtime_path_old_path"></Trans>
          </div>
          <div class="bg-darkSlate-900 p-4 mt-4">
            <div>{currentRuntimePath()?.replaceAll("\\\\", "/")}</div>
          </div>
        </div>
        <div class="h-h-full">
          <div class="text-green-400 font-bold">
            <Trans key="settings:runtime_path_new_path"></Trans>
          </div>
          <div class="bg-darkSlate-900 p-4 mt-4">
            <div>{props.data.runtimePath.replaceAll("\\\\", "/")}</div>
          </div>
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
            loading={props.data.isChangingRuntimePath()}
            disabled={props.data.isChangingRuntimePath()}
            onClick={async () => {
              props.data.setIsChangingRuntimePath(true);
              await window.changeRuntimePath(props.data.runtimePath);
              props.data.setIsChangingRuntimePath(false);
              modalsContext?.closeModal();
            }}
          >
            {t("settings:confirm_change_confirm_button")}
          </Button>
        </div>
      </div>
    </ModalLayout>
  );
};

export default ConfirmChangeRuntimePath;
