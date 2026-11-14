import { ModalProps, useModal } from "../.."
import ModalLayout from "../../ModalLayout"
import { Button, Progress, Radio } from "@gd/ui"
import { Trans, useTransContext } from "@gd/i18n"
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
import type { RuntimeBlocker } from "../../../../../../main/utils/coreShutdown"
import type { UnsupportedRuntimeEntries } from "../../../../../../main/utils/runtimeRelocation"
import { DeletionNoteBox, DeletionNoteLine } from "../DeletionNote"
import {
  isChangingRuntimePath,
  setIsChangingRuntimePath,
  runtimeRecoveryRequired,
  setRuntimeRecoveryRequired
} from "./state"

const blockerLabel = {
  game: "java:_trn_runtime_relocation_busy.game",
  server: "java:_trn_runtime_relocation_busy.server",
  install: "java:_trn_runtime_relocation_busy.install",
  deletion: "java:_trn_runtime_relocation_busy.deletion",
  hook: "java:_trn_runtime_relocation_busy.hook",
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
  "hook",
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
  "hook",
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
  const submitted = {
    target: props.data.runtimePath as string,
    switchOnly: !!props.data.isTargetFolderAlreadyUsed
  }
  const [shutdownUnresolved, setShutdownUnresolved] = createSignal(false)
  // The new folder kept a partial copy; deleting it is the way to retry there.
  const [incompleteTarget, setIncompleteTarget] = createSignal(false)
  // The move stopped and removed what it had copied: the new folder is as it was.
  const [copyRemoved, setCopyRemoved] = createSignal(false)
  // What the core named when it refused to start the move. Empty when it
  // refused without a list: nothing was moved, so the same calm view fits.
  const [busy, setBusy] = createSignal<RuntimeBlocker[]>()
  // The busy view's Restart failed; nothing was moved, so the view stays.
  const [restartFailed, setRestartFailed] = createSignal(false)
  // Stopped before the core was asked to stop: nothing was moved or changed.
  const [stoppedBeforeMove, setStoppedBeforeMove] = createSignal(false)
  // The links or special files that keep the data folder from moving.
  const [unsupported, setUnsupported] =
    createSignal<UnsupportedRuntimeEntries>()
  // An earlier unresolved attempt keeps its recovery view, whatever this one found.
  const notMoved = () => stoppedBeforeMove() && !runtimeRecoveryRequired()

  const [migrationError, setMigrationError] = createSignal<string | undefined>(
    undefined
  )

  const [currentRuntimePath] = createResource(() => {
    return window.getRuntimePath()
  })

  const [t] = useTransContext()
  // What a switch would open, and what in the old folder is not the
  // launcher's. Nothing can be confirmed until this is known; if it cannot
  // be read, nothing is shown and a move leaves those items where they are.
  const [change] = createResource(async () => {
    try {
      return await window.describeRuntimePathChange(submitted.target)
    } catch {
      return undefined
    }
  })
  const found = () => (submitted.switchOnly ? change()?.found : undefined)
  const unfamiliarCount = () =>
    submitted.switchOnly ? 0 : (change()?.unfamiliarCount ?? 0)
  const unfamiliar = () => change()?.unfamiliar ?? []
  // Only this choice makes a move take what is not the launcher's.
  const [moveUnfamiliar, setMoveUnfamiliar] = createSignal(false)
  const locked = () => isChangingRuntimePath() || runtimeRecoveryRequired()

  const attemptRelocation = async (retry: boolean) => {
    if (
      isChangingRuntimePath() ||
      (runtimeRecoveryRequired() && (!retry || !shutdownUnresolved()))
    )
      return
    setIsChangingRuntimePath(true)
    setMigrationError(undefined)
    setIncompleteTarget(false)
    setCopyRemoved(false)
    setBusy(undefined)
    setRestartFailed(false)
    setStoppedBeforeMove(false)
    setUnsupported(undefined)
    try {
      const result = await window.changeRuntimePath(
        submitted.target,
        submitted.switchOnly,
        !submitted.switchOnly && moveUnfamiliar()
      )
      if (result.type === "failed") {
        if (result.refused) {
          // The live core confirmed it holds no move for us, also after an
          // earlier unresolved attempt: nothing is frozen, so dismissing is safe.
          setRuntimeRecoveryRequired(false)
          setShutdownUnresolved(false)
          setBusy(result.refused.blockers)
        } else if (result.recovery !== "none") {
          // Once shutdown ownership is unresolved, a later validation/IPC error
          // cannot silently grant dismissal of the stopped/draining service.
          setRuntimeRecoveryRequired(true)
          setShutdownUnresolved(result.recovery === "shutdown_unresolved")
        }
        setIncompleteTarget(!!result.incompleteTarget)
        setStoppedBeforeMove(
          result.recovery === "none" && !result.incompleteTarget
        )
        setUnsupported(result.unsupported)
        setCopyRemoved(!!result.copyRemoved)
        setMigrationError(result.message)
      } else modalsContext?.closeModal()
    } catch (error) {
      setMigrationError(String(error))
    } finally {
      RTsetProgress(undefined)
      setIsChangingRuntimePath(false)
    }
  }

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
        title={
          submitted.switchOnly
            ? t("java:_trn_runtime_switch.title")
            : props.title
        }
        preventClose={locked()}
        width="w-180"
      >
        <div class="flex flex-col gap-4" data-testid="runtime-path-dialog">
          {/* The window may be lower than all of this; the buttons stay in view. */}
          <div class="flex max-h-[60vh] flex-col gap-4 overflow-y-auto pr-1">
            <div>
              <Switch>
                <Match when={submitted.switchOnly}>
                  <Trans key="java:_trn_runtime_switch.text" />
                </Match>
                <Match when={!submitted.switchOnly}>
                  <Trans key="java:_trn_confirm_change_runtime_path_text" />
                </Match>
              </Switch>
            </div>
            <div>
              <div class="font-bold text-red-400">
                <Trans
                  key={
                    submitted.switchOnly
                      ? "java:_trn_runtime_switch.current_folder"
                      : "java:_trn_runtime_path_old_path"
                  }
                />
              </div>
              <div class="bg-darkSlate-900 mt-2 break-all rounded-md p-3">
                {currentRuntimePath()?.replaceAll("\\\\", "/")}
              </div>
            </div>
            <div>
              <div
                class="font-bold"
                classList={{
                  "text-green-400": !submitted.switchOnly,
                  "text-yellow-400": submitted.switchOnly
                }}
              >
                <Trans
                  key={
                    submitted.switchOnly
                      ? "java:_trn_runtime_switch.new_folder"
                      : "java:_trn_runtime_path_new_path"
                  }
                />
              </div>
              <div class="bg-darkSlate-900 mt-2 break-all rounded-md p-3">
                {submitted.target.replaceAll("\\\\", "/")}
              </div>
            </div>
            <Show when={submitted.switchOnly}>
              <DeletionNoteBox testId="runtime-switch-notes">
                <Show when={found()}>
                  {(data) => (
                    <>
                      <DeletionNoteLine tone="green">
                        <Trans
                          key="java:_trn_runtime_switch.found_instances"
                          options={{ number: data().instances }}
                        />
                      </DeletionNoteLine>
                      <DeletionNoteLine tone="green">
                        <Trans
                          key="java:_trn_runtime_switch.found_servers"
                          options={{ number: data().servers }}
                        />
                      </DeletionNoteLine>
                      <Show
                        when={data().database}
                        fallback={
                          <DeletionNoteLine
                            tone="yellow"
                            testId="runtime-switch-no-database"
                          >
                            <Trans key="java:_trn_runtime_switch.no_database" />
                          </DeletionNoteLine>
                        }
                      >
                        <DeletionNoteLine tone="green">
                          <Trans key="java:_trn_runtime_switch.found_database" />
                        </DeletionNoteLine>
                      </Show>
                    </>
                  )}
                </Show>
                <DeletionNoteLine tone="grey">
                  <Trans key="java:_trn_runtime_switch.nothing_moved" />
                </DeletionNoteLine>
              </DeletionNoteBox>
            </Show>
            <Show when={unfamiliarCount() > 0}>
              <div data-testid="runtime-unfamiliar">
                <div>
                  <Trans
                    key={
                      unfamiliarCount() === 1
                        ? "java:_trn_runtime_unfamiliar.title_single"
                        : "java:_trn_runtime_unfamiliar.title"
                    }
                    options={{ number: unfamiliarCount() }}
                  />
                </div>
                <ul class="bg-darkSlate-900 mt-2 max-h-24 overflow-y-auto rounded-md p-3 text-sm">
                  <For each={unfamiliar()}>
                    {(name) => (
                      <li class="break-all" style={selectable}>
                        {name}
                      </li>
                    )}
                  </For>
                  <Show when={unfamiliarCount() > unfamiliar().length}>
                    <li class="text-lightSlate-600">
                      <Trans
                        key="java:_trn_runtime_unfamiliar.more"
                        options={{
                          number: unfamiliarCount() - unfamiliar().length
                        }}
                      />
                    </li>
                  </Show>
                </ul>
                <div class="mt-2 flex flex-col gap-1">
                  <div data-testid="runtime-unfamiliar-leave">
                    <Radio
                      name="runtime-unfamiliar"
                      value="leave"
                      checked={!moveUnfamiliar()}
                      disabled={locked()}
                      onChange={() => setMoveUnfamiliar(false)}
                    >
                      <div class="font-bold">
                        <Trans key="java:_trn_runtime_unfamiliar.leave" />
                      </div>
                      <div class="text-lightSlate-600 text-sm">
                        <Trans key="java:_trn_runtime_unfamiliar.leave_text" />
                      </div>
                    </Radio>
                  </div>
                  <div data-testid="runtime-unfamiliar-move">
                    <Radio
                      name="runtime-unfamiliar"
                      value="move"
                      checked={moveUnfamiliar()}
                      disabled={locked()}
                      onChange={() => setMoveUnfamiliar(true)}
                    >
                      <div class="font-bold">
                        <Trans key="java:_trn_runtime_unfamiliar.move" />
                      </div>
                      <div class="text-lightSlate-600 text-sm">
                        <Trans key="java:_trn_runtime_unfamiliar.move_text" />
                      </div>
                    </Radio>
                  </div>
                </div>
              </div>
            </Show>
          </div>
          <div class="flex w-full justify-between">
            <Button
              disabled={locked()}
              onClick={() => {
                if (locked()) return
                modalsContext?.closeModal()
              }}
            >
              <Trans key="settings:_trn_confirm_change_cancel_button" />
            </Button>
            <Button
              type="secondary"
              disabled={locked() || change.loading}
              onClick={() => {
                if (change.loading) return
                void attemptRelocation(false)
              }}
            >
              <Trans
                key={
                  submitted.switchOnly
                    ? "java:_trn_runtime_switch.confirm"
                    : "settings:_trn_confirm_change_confirm_button"
                }
              />
            </Button>
          </div>
        </div>
      </ModalLayout>

      <Show when={isChangingRuntimePath() || migrationError()}>
        <Portal mount={document.getElementById("overlay")!}>
          <div
            class="z-9999 fixed inset-0 flex flex-col items-center justify-center bg-opacity-65 p-8 backdrop-blur-sm"
            classList={{
              "bg-black": !migrationError() || !!busy() || notMoved(),
              "bg-red-900": !!migrationError() && !busy() && !notMoved()
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
                    <Show
                      when={notMoved()}
                      fallback={
                        /* A copy that stopped means nothing was published or removed: say so calmly. */
                        <Trans
                          key={
                            incompleteTarget() || copyRemoved()
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
                      }
                    >
                      <Trans key="java:_trn_runtime_recovery.not_moved_title" />
                    </Show>
                  </div>
                  <Show when={notMoved()}>
                    <p class="mt-4 text-base">
                      <Trans key="java:_trn_runtime_recovery.not_moved_body" />
                    </p>
                  </Show>
                  <Show
                    when={unsupported()}
                    fallback={
                      <div class="mt-4 text-base opacity-80">
                        {migrationError()}
                      </div>
                    }
                  >
                    {(found) => (
                      <div data-testid="relocation-unsupported">
                        <p class="mt-4 text-base">
                          <Trans key="java:_trn_runtime_recovery.links_inside" />
                        </p>
                        <ul class="bg-darkSlate-900 mt-4 max-h-48 overflow-y-auto rounded-md p-4 text-left">
                          <For each={found().entries}>
                            {(entry) => (
                              <li class="break-all text-xs" style={selectable}>
                                {entry}
                              </li>
                            )}
                          </For>
                          <Show when={found().count > found().entries.length}>
                            <li class="text-xs">
                              <Trans
                                key="java:_trn_runtime_recovery.left_behind_more"
                                options={{
                                  number: found().count - found().entries.length
                                }}
                              />
                            </li>
                          </Show>
                        </ul>
                        <div class="mt-4 flex justify-center gap-4">
                          <Button
                            type="secondary"
                            onClick={() => window.openFolder(found().folder)}
                          >
                            <Trans key="java:_trn_runtime_recovery.open_data_folder" />
                          </Button>
                          <Show when={notMoved()}>
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
                  </Show>
                  <Show when={incompleteTarget()}>
                    <p class="mt-4">
                      <Trans
                        key="java:_trn_runtime_recovery.incomplete_target"
                        options={{
                          path: submitted.target,
                          // Rendered as text; escaping would show "&#x2F;" for "/".
                          interpolation: { escapeValue: false }
                        }}
                      />
                    </p>
                  </Show>
                  <Show when={copyRemoved()}>
                    <p class="mt-4">
                      <Trans
                        key="java:_trn_runtime_recovery.copy_removed"
                        options={{
                          path: submitted.target,
                          // Rendered as text; escaping would show "&#x2F;" for "/".
                          interpolation: { escapeValue: false }
                        }}
                      />
                    </p>
                  </Show>
                  <Show when={runtimeRecoveryRequired()}>
                    <Show when={!incompleteTarget() && !copyRemoved()}>
                      <p>
                        <Trans key="java:_trn_runtime_recovery.keep_folders" />
                      </p>
                    </Show>
                    <Show when={shutdownUnresolved()}>
                      <p>
                        <Trans key="java:_trn_runtime_recovery.unresolved" />
                      </p>
                      <Button
                        disabled={isChangingRuntimePath()}
                        onClick={() => attemptRelocation(true)}
                      >
                        <Trans key="java:_trn_runtime_recovery.retry" />
                      </Button>
                    </Show>
                    <Show when={!shutdownUnresolved()}>
                      <Button
                        type="secondary"
                        disabled={isChangingRuntimePath()}
                        onClick={async () => {
                          if (isChangingRuntimePath()) return
                          setIsChangingRuntimePath(true)
                          try {
                            await window.relaunch()
                          } catch (error) {
                            setMigrationError(String(error))
                          } finally {
                            setIsChangingRuntimePath(false)
                          }
                        }}
                      >
                        <Trans key="java:_trn_runtime_recovery.restart" />
                      </Button>
                    </Show>
                  </Show>
                  <Show when={!runtimeRecoveryRequired()}>
                    <Button
                      type="secondary"
                      class="mt-6"
                      onClick={() => {
                        if (runtimeRecoveryRequired()) return
                        setMigrationError(undefined)
                        modalsContext?.closeModal()
                      }}
                    >
                      <Trans key="general:_trn_dismiss" />
                    </Button>
                  </Show>
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
