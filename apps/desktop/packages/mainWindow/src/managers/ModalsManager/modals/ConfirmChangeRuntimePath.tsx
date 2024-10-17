import { ModalProps, useModal } from "..";
import ModalLayout from "../ModalLayout";
import { Button, Progressbar } from "@gd/ui";
import { Trans } from "@gd/i18n";
import { Match, Show, Switch, createResource, createSignal } from "solid-js";
import { Portal } from "solid-js/web";
import { RTprogress, RTsetProgress } from "@/utils/runtimePathProgress";

const ConfirmChangeRuntimePath = (props: ModalProps) => {
  const modalsContext = useModal();

  const [migrationError, setMigrationError] = createSignal<string | undefined>(
    undefined
  );

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
            <Switch>
              <Match when={props.data.isTargetFolderAlreadyUsed}>
                <Trans key="settings:confirm_change_runtime_path_already_used_text" />
              </Match>
              <Match when={!props.data.isTargetFolderAlreadyUsed}>
                <Trans key="settings:confirm_change_runtime_path_text" />
              </Match>
            </Switch>
          </div>
          <div class="h-h-full">
            <div class="font-bold text-red-400">
              <Trans key="settings:runtime_path_old_path" />
            </div>
            <div class="bg-darkSlate-900 p-4 mt-4">
              <div>{currentRuntimePath()?.replaceAll("\\\\", "/")}</div>
            </div>
          </div>
          <div class="h-h-full">
            <div
              class="font-bold"
              classList={{
                "text-green-400": !props.data.isTargetFolderAlreadyUsed,
                "text-yellow-400": props.data.isTargetFolderAlreadyUsed
              }}
            >
              <Trans key="settings:runtime_path_new_path" />
            </div>
            <div
              class="bg-darkSlate-900 p-4 mt-4"
              classList={{
                "text-yellow-400": props.data.isTargetFolderAlreadyUsed
              }}
            >
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

                try {
                  await window.changeRuntimePath(props.data.runtimePath);
                  modalsContext?.closeModal();
                } catch (e: any) {
                  setMigrationError(e.message);
                }

                RTsetProgress(undefined);
                props.data.setIsChangingRuntimePath(false);
              }}
            >
              <Trans key="settings:confirm_change_confirm_button" />
            </Button>
          </div>
        </div>
      </ModalLayout>
      <Show when={props.data.isChangingRuntimePath() || migrationError()}>
        <Portal>
          <div
            class="inset-0 z-100 backdrop-blur-sm flex flex-col items-center justify-center fixed bg-opacity-65 p-8"
            classList={{
              "bg-black": !migrationError(),
              "bg-red-900": !!migrationError()
            }}
          >
            <div>
              <Switch>
                <Match when={migrationError()}>
                  <div class="flex text-2xl items-center text-center">
                    <div>
                      <Trans key="settings:migration_errored">
                        {""}
                        <span
                          class="underline cursor-pointer text-lightSlate-50 hover:text-lightSlate-400"
                          onClick={() => {
                            window.openExternalLink(
                              "https://gdlauncher.com/docs/troubleshooting/#migration-error"
                            );
                          }}
                        />
                        {""}
                      </Trans>
                    </div>
                  </div>
                </Match>
                <Match when={!migrationError()}>
                  <div class="flex text-2xl items-center">
                    <Trans key="settings:applying_new_runtime_path" />
                    <div class="ml-2 i-ri:loader-4-line animate-spin" />
                  </div>
                </Match>
              </Switch>
            </div>
            <div class="mt-4">
              <Switch>
                <Match when={migrationError()}>{migrationError()}</Match>
                <Match when={!migrationError()}>
                  <Trans key="settings:do_not_close_app" />
                </Match>
              </Switch>
              <div
                class="mt-4 text-lightSlate-400"
                classList={{
                  "opacity-0": RTprogress() === undefined
                }}
              >
                <div>
                  {RTprogress()?.current} / {RTprogress()?.total}
                </div>
                <div>
                  <Switch>
                    <Match when={RTprogress()?.action === "copy"}>
                      <Trans
                        key="settings:copying_file"
                        options={{
                          file: RTprogress()?.currentName
                        }}
                      />
                    </Match>
                    <Match when={RTprogress()?.action === "remove"}>
                      <Trans
                        key="settings:removing_file"
                        options={{
                          file: RTprogress()?.currentName
                        }}
                      />
                    </Match>
                  </Switch>
                </div>

                <div class="w-full">
                  <Progressbar
                    color="bg-primary-400"
                    percentage={
                      RTprogress()
                        ? (RTprogress()!.current * 100) / RTprogress()!.total
                        : 0
                    }
                  />
                </div>
              </div>
            </div>
          </div>
        </Portal>
      </Show>
    </>
  );
};

export default ConfirmChangeRuntimePath;
