import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { Button, Progress } from "@gd/ui"
import { Trans } from "@gd/i18n"
import { Match, Show, Switch, createResource, createSignal } from "solid-js"
import { Portal } from "solid-js/web"
import { RTprogress, RTsetProgress } from "@/utils/runtimePathProgress"

const ConfirmChangeRuntimePath = (props: ModalProps) => {
  const modalsContext = useModal()

  const [migrationError, setMigrationError] = createSignal<string | undefined>(
    undefined
  )

  const [currentRuntimePath] = createResource(() => {
    return window.getRuntimePath()
  })

  return (
    <>
      <ModalLayout
        noHeader={props.noHeader}
        title={props.title}
        height="h-120"
        width="w-180"
      >
        <div class="flex h-full flex-col justify-between">
          <div class="h-h-full">
            <Switch>
              <Match when={props.data.isTargetFolderAlreadyUsed}>
                <Trans key="java:_trn_confirm_change_runtime_path_already_used_text" />
              </Match>
              <Match when={!props.data.isTargetFolderAlreadyUsed}>
                <Trans key="java:_trn_confirm_change_runtime_path_text" />
              </Match>
            </Switch>
          </div>
          <div class="h-h-full">
            <div class="font-bold text-red-400">
              <Trans key="java:_trn_runtime_path_old_path" />
            </div>
            <div class="bg-darkSlate-900 mt-4 p-4">
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
              <Trans key="java:_trn_runtime_path_new_path" />
            </div>
            <div
              class="bg-darkSlate-900 mt-4 p-4"
              classList={{
                "text-yellow-400": props.data.isTargetFolderAlreadyUsed
              }}
            >
              <div>{props.data.runtimePath.replaceAll("\\\\", "/")}</div>
            </div>
          </div>
          <div class="flex w-full justify-between">
            <Button
              onClick={() => {
                modalsContext?.closeModal()
              }}
            >
              <Trans key="settings:_trn_confirm_change_cancel_button" />
            </Button>
            <Button
              type="secondary"
              disabled={props.data.isChangingRuntimePath()}
              onClick={async () => {
                props.data.setIsChangingRuntimePath(true)

                try {
                  await window.changeRuntimePath(props.data.runtimePath)
                  modalsContext?.closeModal()
                } catch (e: any) {
                  setMigrationError(e.message)
                }

                RTsetProgress(undefined)
                props.data.setIsChangingRuntimePath(false)
              }}
            >
              <Trans key="settings:_trn_confirm_change_confirm_button" />
            </Button>
          </div>
        </div>
      </ModalLayout>
      <Show when={props.data.isChangingRuntimePath() || migrationError()}>
        <Portal>
          <div
            class="z-100 fixed inset-0 flex flex-col items-center justify-center bg-opacity-65 p-8 backdrop-blur-sm"
            classList={{
              "bg-black": !migrationError(),
              "bg-red-900": !!migrationError()
            }}
          >
            <div>
              <Switch>
                <Match when={migrationError()}>
                  <div class="flex items-center text-center text-2xl">
                    <div>
                      <Trans key="java:_trn_migration_errored">
                        {""}
                        <span
                          class="text-lightSlate-50 hover:text-lightSlate-400 cursor-pointer underline"
                          onClick={() => {
                            window.openExternalLink(
                              "https://gdlauncher.com/docs/troubleshooting/#migration-error"
                            )
                          }}
                        />
                        {""}
                      </Trans>
                    </div>
                  </div>
                </Match>
                <Match when={!migrationError()}>
                  <div class="flex items-center text-2xl">
                    <Trans key="java:_trn_applying_new_runtime_path" />
                    <div class="i-hugeicons:loading-03 ml-2 animate-spin" />
                  </div>
                </Match>
              </Switch>
            </div>
            <div class="mt-4">
              <Switch>
                <Match when={migrationError()}>{migrationError()}</Match>
                <Match when={!migrationError()}>
                  <Trans key="java:_trn_do_not_close_app" />
                </Match>
              </Switch>
              <div
                class="text-lightSlate-400 mt-4"
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
                        key="java:_trn_copying_file"
                        options={{ file: RTprogress()?.currentName }}
                      />
                    </Match>
                    <Match when={RTprogress()?.action === "remove"}>
                      <Trans
                        key="java:_trn_removing_file"
                        options={{ file: RTprogress()?.currentName }}
                      />
                    </Match>
                  </Switch>
                </div>

                <div class="w-full">
                  <Progress
                    color="bg-primary-400"
                    value={
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
  )
}

export default ConfirmChangeRuntimePath
