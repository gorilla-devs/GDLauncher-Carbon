import { ModalProps, useModal } from "..";
import ModalLayout from "../ModalLayout";
import { Button } from "@gd/ui";
import { Trans } from "@gd/i18n";
import { Show, createResource } from "solid-js";
import { Portal } from "solid-js/web";

const ConfirmChangeRuntimePath = (props: ModalProps) => {
  const modalsContext = useModal();

  const [currentRuntimePath] = createResource(() => {
    return window.getRuntimePath();
  });

  return (
    <>
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
              <Trans key="settings:runtime_path_old_path" />
            </div>
            <div class="bg-darkSlate-900 p-4 mt-4">
              <div>{currentRuntimePath()?.replaceAll("\\\\", "/")}</div>
            </div>
          </div>
          <div class="h-h-full">
            <div class="text-green-400 font-bold">
              <Trans key="settings:runtime_path_new_path" />
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
              <Trans key="settings:confirm_change_cancel_button" />
            </Button>
            <Button
              type="secondary"
              disabled={props.data.isChangingRuntimePath()}
              onClick={async () => {
                props.data.setIsChangingRuntimePath(true);
                await window.changeRuntimePath(props.data.runtimePath);
                props.data.setIsChangingRuntimePath(false);
                modalsContext?.closeModal();
              }}
            >
              <Trans key="settings:confirm_change_confirm_button" />
            </Button>
          </div>
        </div>
      </ModalLayout>
      <Show when={props.data.isChangingRuntimePath()}>
        <Portal>
          <div class="fixed inset-0 bg-black z-100 backdrop-blur-sm flex flex-col items-center justify-center bg-opacity-65">
            <div class="flex text-2xl items-center">
              <Trans key="settings:applying_new_runtime_path" />
              <div class="i-ri:loader-4-line animate-spin ml-2" />
            </div>
            <div class="mt-4">
              <Trans key="settings:do_not_close_app" />
            </div>
          </div>
        </Portal>
      </Show>
    </>
  );
};

export default ConfirmChangeRuntimePath;
