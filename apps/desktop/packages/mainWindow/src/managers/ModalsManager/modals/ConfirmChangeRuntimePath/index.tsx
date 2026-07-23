import { ModalProps, useModal } from "../.."
import ModalLayout from "../../ModalLayout"
import { Button, Progress } from "@gd/ui"
import { Trans } from "@gd/i18n"
import { Match, Show, Switch, createResource, createSignal } from "solid-js"
import { Portal } from "solid-js/web"
import { RTprogress, RTsetProgress } from "@/utils/runtimePathProgress"
import { isChangingRuntimePath, setIsChangingRuntimePath } from "./state"

const ConfirmChangeRuntimePath = (props: ModalProps) => {
  const modalsContext = useModal()

  const [migrationError, setMigrationError] = createSignal<string | undefined>(
    undefined
  )

  const [currentRuntimePath] = createResource(() => {
    return window.getRuntimePath()
  })

  const progressValue = () => {
    const p = RTprogress()
    if (!p || p.action === "scan" || p.total === 0) return 0
    return (p.current * 100) / p.total
  }

  return (
    <>
      <ModalLayout
        noHeader={props.noHeader}
        title={props.title}
        preventClose={isChangingRuntimePath()}
        height="h-120"
        width="w-180"
      >
        <div class="flex h-full flex-col justify-between">
          <div>
            <Switch>
              <Match when={props.data.isTargetFolderAlreadyUsed}>
                <Trans key="java:_trn_confirm_change_runtime_path_already_used_text" />
              </Match>
              <Match when={!props.data.isTargetFolderAlreadyUsed}>
                <Trans key="java:_trn_confirm_change_runtime_path_text" />
              </Match>
            </Switch>
          </div>
          <div>
            <div class="font-bold text-red-400">
              <Trans key="java:_trn_runtime_path_old_path" />
            </div>
            <div class="bg-darkSlate-900 mt-4 break-all p-4">
              {currentRuntimePath()?.replaceAll("\\\\", "/")}
            </div>
          </div>
          <div>
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
              class="bg-darkSlate-900 mt-4 break-all p-4"
              classList={{
                "text-yellow-400": props.data.isTargetFolderAlreadyUsed
              }}
            >
              {props.data.runtimePath.replaceAll("\\\\", "/")}
            </div>
          </div>
          <div class="flex w-full justify-between">
            <Button
              disabled={isChangingRuntimePath()}
              onClick={() => {
                modalsContext?.closeModal()
              }}
            >
              <Trans key="settings:_trn_confirm_change_cancel_button" />
            </Button>
            <Button
              type="secondary"
              disabled={isChangingRuntimePath()}
              onClick={async () => {
                setIsChangingRuntimePath(true)
                setMigrationError(undefined)

                try {
                  await window.changeRuntimePath(
                    props.data.runtimePath,
                    props.data.isTargetFolderAlreadyUsed
                  )
                  modalsContext?.closeModal()
                } catch (e: any) {
                  setMigrationError(e.message)
                }

                RTsetProgress(undefined)
                setIsChangingRuntimePath(false)
              }}
            >
              <Trans key="settings:_trn_confirm_change_confirm_button" />
            </Button>
          </div>
        </div>
      </ModalLayout>

      <Show when={isChangingRuntimePath() || migrationError()}>
        <Portal mount={document.getElementById("overlay")!}>
          <div
            class="z-9999 fixed inset-0 flex flex-col items-center justify-center bg-opacity-65 p-8 backdrop-blur-sm"
            classList={{
              "bg-black": !migrationError(),
              "bg-red-900": !!migrationError()
            }}
          >
            <Switch>
              <Match when={migrationError()}>
                <div class="max-w-2xl text-center">
                  <div class="text-2xl">
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
                  <div class="mt-4 text-base opacity-80">
                    {migrationError()}
                  </div>
                  <Button
                    type="secondary"
                    class="mt-6"
                    onClick={() => {
                      setMigrationError(undefined)
                      modalsContext?.closeModal()
                    }}
                  >
                    <Trans key="general:_trn_dismiss" />
                  </Button>
                </div>
              </Match>
              <Match when={!migrationError()}>
                <div class="flex items-center text-2xl">
                  <Trans key="java:_trn_applying_new_runtime_path" />
                  <div class="i-hugeicons:loading-03 ml-2 animate-spin" />
                </div>

                <div class="text-yellow-300 mt-6 max-w-2xl text-center font-bold">
                  <Trans key="java:_trn_do_not_close_app_or_pc_title" />
                </div>
                <div class="text-lightSlate-200 mt-2 max-w-2xl text-center text-sm">
                  <Trans key="java:_trn_do_not_close_app_or_pc_body" />
                </div>

                <div class="text-lightSlate-400 mt-8 w-full max-w-2xl">
                  <div class="text-center text-sm">
                    <Switch>
                      <Match when={RTprogress()?.action === "scan"}>
                        <Trans key="java:_trn_scanning_files" />
                      </Match>
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

                  <Show when={RTprogress() && RTprogress()!.action !== "scan"}>
                    <div class="mt-2 text-center text-xs">
                      {RTprogress()!.current} / {RTprogress()!.total}
                    </div>
                  </Show>

                  <div class="mt-2 w-full">
                    <Progress color="bg-primary-400" value={progressValue()} />
                  </div>
                </div>
              </Match>
            </Switch>
          </div>
        </Portal>
      </Show>
    </>
  )
}

export default ConfirmChangeRuntimePath
