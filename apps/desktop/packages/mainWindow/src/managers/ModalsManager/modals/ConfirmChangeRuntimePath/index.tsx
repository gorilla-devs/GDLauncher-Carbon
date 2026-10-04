import { ModalProps, useModal } from "../.."
import ModalLayout from "../../ModalLayout"
import { Button, Progress } from "@gd/ui"
import { Trans } from "@gd/i18n"
import {
  For,
  Match,
  Show,
  Switch,
  createResource,
  createSignal
} from "solid-js"
import { Portal } from "solid-js/web"
import { RTprogress, RTsetProgress } from "@/utils/runtimePathProgress"
import { isChangingRuntimePath, setIsChangingRuntimePath } from "./state"

const blockerLabel = {
  game: "java:_trn_runtime_relocation_busy.game",
  server: "java:_trn_runtime_relocation_busy.server",
  install: "java:_trn_runtime_relocation_busy.install",
  deletion: "java:_trn_runtime_relocation_busy.deletion",
  task: "java:_trn_runtime_relocation_busy.task",
  unconfirmed: "java:_trn_runtime_relocation_busy.unconfirmed",
  previousGame: "java:_trn_runtime_relocation_busy.previous_game",
  previousProcess: "java:_trn_runtime_relocation_busy.previous_process",
  unreadableRecord: "java:_trn_runtime_relocation_busy.unreadable_record",
  startupCheckPending:
    "java:_trn_runtime_relocation_busy.startup_check_pending",
  startupCheckFailed: "java:_trn_runtime_relocation_busy.startup_check_failed",
  shutdown: "java:_trn_runtime_relocation_busy.shutdown",
  relocation: "java:_trn_runtime_relocation_busy.relocation"
} as const satisfies Record<RuntimeBlocker["kind"], string>
// A kind this build does not know yet still gets a plain label.
const labelFor = (kind: string) =>
  Object.hasOwn(blockerLabel, kind)
    ? blockerLabel[kind as RuntimeBlocker["kind"]]
    : blockerLabel.task
/**
 * Running work that the plain "close Minecraft, stop your servers and let
 * downloads finish" body covers. Any other row gets the "here is what to do"
 * body; every label says what to do about it, so a mixed list keeps each step.
 */
const closable = new Set<string>([
  "game",
  "server",
  "install",
  "deletion",
  "task",
  "previousGame"
])
/**
 * Only restarting GDLauncher clears these; Try again never will. A failed
 * startup check never holds up a quit or restart, so the restart goes
 * through. An unreadable record is not listed: Try again reads its file
 * again, so deleting it, or a lock on it lifting, clears it without a
 * restart, and so does a launch of its instance once the game exits. An
 * unconfirmed task is not listed either: it can still settle on its own,
 * and restarting with one left ends unresolved.
 */
const restartOnly = new Set<string>(["startupCheckFailed"])
/**
 * For running work the name is an internal owner ("game 3", a function
 * name), so only its label is shown. The other kinds, and any kind this build
 * does not know, name a folder, a process or an error the user can act on.
 */
const hiddenName = new Set<string>([
  "game",
  "server",
  "install",
  "deletion",
  "task",
  "unconfirmed",
  "startupCheckPending",
  "shutdown",
  "relocation"
])
const fileName = (path: string) => path.split(/[\\/]/).at(-1) ?? ""
/**
 * A launch of the instance replaces its marker and the game's exit removes
 * it, so that file has a way out that needs no file manager.
 */
const isInstanceMarker = (path: string) =>
  fileName(path) === ".gdl_instance.pid"
/**
 * A server's folder (`<runtime>/servers/<name>`) and its marker. The server
 * page can stop a server left running by an earlier session, letting it save
 * where the system allows, or clear its record, so those rows point there
 * before Task Manager or a reboot.
 */
const isServerFolder = (path: string) =>
  path.split(/[\\/]/).filter(Boolean).at(-2) === "servers"
const isServerMarker = (path: string) => fileName(path) === ".gdl_server.pid"
/** Finder and most Linux file managers hide dot files; Explorer does not. */
const hiddenByFileManager = (path: string) =>
  fileName(path).startsWith(".") && !navigator.platform.startsWith("Win")
/**
 * Everything else here is not selectable (index.html), but a path is
 * something to copy into a file manager or a support request.
 */
const selectable = { "user-select": "text", cursor: "text" } as const
/** One row per label and detail: two servers are one "a server is running". */
function blockerRows(blockers: RuntimeBlocker[]) {
  const rows = new Map<
    string,
    {
      kind: string
      detail: string
      pid: number | undefined
      program: string | undefined
    }
  >()
  for (const blocker of blockers) {
    const detail = hiddenName.has(blocker.kind) ? "" : blocker.name
    // What Task Manager and Activity Monitor list a leftover process by.
    const pid = typeof blocker.pid === "number" ? blocker.pid : undefined
    // The program holding that PID now, so the user can tell whether it is
    // Java before ending anything.
    const program =
      pid !== undefined &&
      typeof blocker.program === "string" &&
      blocker.program.trim()
        ? blocker.program.trim()
        : undefined
    rows.set(
      `${labelFor(blocker.kind)}\n${detail}\n${pid ?? ""}\n${program ?? ""}`,
      { kind: blocker.kind, detail, pid, program }
    )
  }
  return [...rows.values()]
}

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
    if (!p || p.action === "scan" || p.action === "check" || p.total === 0)
      return 0
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
              "bg-black": !migrationError() || !!busy(),
              "bg-red-900": !!migrationError() && !busy()
            }}
          >
            <Switch>
              <Match when={migrationError() ? busy() : undefined}>
                {(blockers) => (
                  <div class="max-w-2xl text-center">
                    <div class="text-2xl">
                      <Trans key="java:_trn_runtime_relocation_busy.title" />
                    </div>
                    <p class="mt-4 text-base">
                      <Switch
                        fallback={
                          <Trans key="java:_trn_runtime_relocation_busy.body_steps" />
                        }
                      >
                        <Match when={blockers().length === 0}>
                          <Trans key="java:_trn_runtime_relocation_busy.not_started" />
                        </Match>
                        <Match
                          when={blockers().every((b) => closable.has(b.kind))}
                        >
                          <Trans key="java:_trn_runtime_relocation_busy.body" />
                        </Match>
                      </Switch>
                    </p>
                    <Show when={blockers().length > 0}>
                      <ul class="bg-darkSlate-900 mt-4 flex flex-col gap-2 rounded-md p-4 text-left">
                        <For each={blockerRows(blockers())}>
                          {(row) => (
                            <li>
                              <Trans key={labelFor(row.kind)} />
                              <Show when={row.detail}>
                                <div
                                  class="text-lightSlate-400 break-all text-xs"
                                  style={selectable}
                                >
                                  {row.detail}
                                </div>
                              </Show>
                              <Show when={row.pid !== undefined}>
                                <div
                                  class="text-lightSlate-400 text-xs"
                                  data-testid="relocation-pid"
                                >
                                  <Show
                                    when={row.program}
                                    fallback={
                                      <Trans
                                        key="java:_trn_runtime_relocation_busy.pid"
                                        options={{ pid: row.pid }}
                                      />
                                    }
                                  >
                                    {(program) => (
                                      <Trans
                                        key="java:_trn_runtime_relocation_busy.pid_program"
                                        options={{
                                          pid: row.pid,
                                          program: program(),
                                          // Rendered as text; escaping would show "&amp;" for "&".
                                          interpolation: { escapeValue: false }
                                        }}
                                      />
                                    )}
                                  </Show>
                                </div>
                              </Show>
                              <Show
                                when={
                                  row.kind === "previousProcess" &&
                                  isServerFolder(row.detail)
                                }
                              >
                                <div
                                  class="text-lightSlate-400 mt-1 text-xs"
                                  data-testid="relocation-server-hint"
                                >
                                  <Trans key="java:_trn_runtime_relocation_busy.previous_server" />
                                </div>
                              </Show>
                              <Show
                                when={
                                  row.kind === "unreadableRecord" && row.detail
                                }
                              >
                                {(file) => (
                                  <>
                                    <Show when={isInstanceMarker(file())}>
                                      <div class="text-lightSlate-400 mt-1 text-xs">
                                        <Trans key="java:_trn_runtime_relocation_busy.unreadable_instance_marker" />
                                      </div>
                                    </Show>
                                    <Show when={isServerMarker(file())}>
                                      <div
                                        class="text-lightSlate-400 mt-1 text-xs"
                                        data-testid="relocation-server-hint"
                                      >
                                        <Trans key="java:_trn_runtime_relocation_busy.unreadable_server_marker" />
                                      </div>
                                    </Show>
                                    <Show when={hiddenByFileManager(file())}>
                                      <div class="text-lightSlate-400 mt-1 text-xs">
                                        <Trans key="java:_trn_runtime_relocation_busy.hidden_file" />
                                      </div>
                                    </Show>
                                    <Button
                                      class="mt-2"
                                      size="small"
                                      type="secondary"
                                      onClick={() => window.openFolder(file())}
                                    >
                                      <Trans key="java:_trn_runtime_relocation_busy.show_file" />
                                    </Button>
                                  </>
                                )}
                              </Show>
                            </li>
                          )}
                        </For>
                      </ul>
                    </Show>
                    <Show when={restartFailed()}>
                      <p class="text-yellow-300 mt-4 text-base">
                        <Trans key="java:_trn_runtime_relocation_busy.restart_failed" />
                      </p>
                    </Show>
                    <div class="mt-6 flex justify-center gap-4">
                      <Button
                        type="secondary"
                        disabled={isChangingRuntimePath()}
                        onClick={() => {
                          setMigrationError(undefined)
                          setBusy(undefined)
                          setRestartFailed(false)
                          modalsContext?.closeModal()
                        }}
                      >
                        <Trans key="general:_trn_dismiss" />
                      </Button>
                      <Show
                        when={blockers().some((b) => restartOnly.has(b.kind))}
                      >
                        <Button
                          disabled={isChangingRuntimePath()}
                          onClick={async () => {
                            if (isChangingRuntimePath()) return
                            setIsChangingRuntimePath(true)
                            setRestartFailed(false)
                            try {
                              await window.relaunch()
                            } catch (error) {
                              // Nothing was moved, so the calm view stays and
                              // says so; the raw reason is for the logs only.
                              console.error(
                                "Restart from the move refusal failed",
                                error
                              )
                              setRestartFailed(true)
                            } finally {
                              setIsChangingRuntimePath(false)
                            }
                          }}
                        >
                          <Trans key="java:_trn_runtime_relocation_busy.restart" />
                        </Button>
                      </Show>
                      <Show
                        when={
                          blockers().length === 0 ||
                          !blockers().every((b) => restartOnly.has(b.kind))
                        }
                      >
                        <Button
                          disabled={isChangingRuntimePath()}
                          onClick={() => attemptRelocation(false)}
                        >
                          <Trans key="java:_trn_runtime_relocation_busy.try_again" />
                        </Button>
                      </Show>
                    </div>
                  </div>
                )}
              </Match>
              <Match when={migrationError()}>
                <div class="max-w-2xl text-center">
                  <div class="text-2xl">
                    {/* A partial copy means nothing was published or removed: say so calmly. */}
                    <Trans
                      key={
                        incompleteTarget()
                          ? "java:_trn_migration_stopped"
                          : "java:_trn_migration_errored"
                      }
                    >
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

                {/* Until the core is asked to stop nothing is moved, and closing the
                    app ends the move there (main/index.ts quitUnlessKeptAlive). */}
                <Show
                  when={RTprogress() && RTprogress()!.action !== "check"}
                  fallback={
                    <div class="text-lightSlate-200 mt-6 max-w-2xl text-center text-sm">
                      <Trans key="java:_trn_checking_runtime_path" />
                    </div>
                  }
                >
                  <div class="text-yellow-300 mt-6 max-w-2xl text-center font-bold">
                    <Trans key="java:_trn_do_not_close_app_or_pc_title" />
                  </div>
                  <div class="text-lightSlate-200 mt-2 max-w-2xl text-center text-sm">
                    <Trans key="java:_trn_do_not_close_app_or_pc_body" />
                  </div>
                </Show>

                <div class="text-lightSlate-400 mt-8 w-full max-w-2xl">
                  <div class="text-center text-sm">
                    <Switch>
                      <Match when={RTprogress()?.action === "scan"}>
                        <Trans key="java:_trn_scanning_files" />
                      </Match>
                      <Match when={RTprogress()?.action === "copy"}>
                        <Trans
                          key="java:_trn_copying_file"
                          options={{
                            file: RTprogress()?.currentName,
                            // Rendered as text; escaping would show "&#x2F;" for "/".
                            interpolation: { escapeValue: false }
                          }}
                        />
                      </Match>
                      {/* The copy is complete and is being written to the drive. */}
                      <Match when={RTprogress()?.action === "flush"}>
                        <Trans key="java:_trn_saving_copy" />
                      </Match>
                      <Match when={RTprogress()?.action === "remove"}>
                        <Trans
                          key="java:_trn_removing_file"
                          options={{
                            file: RTprogress()?.currentName,
                            // Rendered as text; escaping would show "&#x2F;" for "/".
                            interpolation: { escapeValue: false }
                          }}
                        />
                      </Match>
                      {/* The move stopped; its copy is removed from the new folder. */}
                      <Match when={RTprogress()?.action === "rollback"}>
                        <Trans
                          key="java:_trn_removing_partial_copy"
                          options={{
                            file: RTprogress()?.currentName,
                            // Rendered as text; escaping would show "&#x2F;" for "/".
                            interpolation: { escapeValue: false }
                          }}
                        />
                      </Match>
                    </Switch>
                  </div>

                  <Show
                    when={
                      RTprogress() &&
                      RTprogress()!.action !== "scan" &&
                      RTprogress()!.action !== "check"
                    }
                  >
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
