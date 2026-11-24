import { useTransContext } from "@gd/i18n"
import { Button, Progress } from "@gd/ui"
import { Show, createSignal, onCleanup, onMount } from "solid-js"
import { rspc, rspcErrorCode } from "@/utils/rspcClient"
import { useFormatSize } from "@/utils/java/useFormatSize"
import type { JavaIssuePrompt } from "@/utils/javaIssues"
import { ModalProps, useModalStackEntry } from "../.."
import ModalLayout from "../../ModalLayout"
import { DialogFooter } from "../../DialogFooter"
import { TextLink } from "@/components/TextLink"
import { progressLabel, progressView } from "./managedJavaCatalog"

/** After a game crashed on a Java older than a mod needs: one button sets
 *  the instance to the oldest Java that works, downloading it when there is
 *  none, and plays. Before a launch on a Java that doesn't fit, the same
 *  button moves it to one that fits, and a quiet link plays anyway. */
const JavaIssueModal = (props: ModalProps) => {
  const [t] = useTransContext()
  const entry = useModalStackEntry()
  const data = () => props.data as JavaIssuePrompt
  const beforeLaunch = () => data().beforeLaunch === true
  const [busy, setBusy] = createSignal(false)
  // The button's width when it was pressed, so its spinner doesn't shrink it.
  const [busyWidth, setBusyWidth] = createSignal<number>()

  const fix = rspc.createMutation(() => ({
    mutationKey: ["instance.fixJavaAndLaunch"]
  }))
  const fixMismatch = rspc.createMutation(() => ({
    mutationKey: ["instance.fixJavaMismatchAndLaunch"]
  }))
  const launch = rspc.createMutation(() => ({
    mutationKey: ["instance.launchInstance"]
  }))

  // The fix carries on after the dialog closes.
  let disposed = false
  onCleanup(() => {
    disposed = true
  })

  // While fixing, the install of the Java the fix downloads, if it does.
  const installs = rspc.createQuery(() => ({
    queryKey: ["java.getManagedJavaInstalls"],
    enabled: busy()
  }))
  const install = () => {
    if (!busy()) return undefined
    const all = installs.data ?? []
    return (
      all.find((i) => i.major === data().fixMajor) ??
      all.find((i) => i.major >= data().neededMajor)
    )
  }
  const progress = rspc.createQuery(() => ({
    queryKey: ["java.getSetupManagedJavaProgress", install()?.buildId ?? ""],
    enabled: install() !== undefined,
    refetchInterval: install() !== undefined ? 300 : false
  }))
  const view = () =>
    progressView(progress.data ?? install()?.progress ?? "idle")
  const percent = () => {
    const fraction = view().fraction
    return fraction !== undefined ? Math.round(fraction * 100) : undefined
  }
  const size = useFormatSize()
  const progressText = () => progressLabel(view(), t, size)

  let primary: HTMLButtonElement | undefined
  onMount(() => primary?.focus())

  // Its own entry: another dialog may have opened on top meanwhile.
  const close = () => entry?.close()

  const play = async () => {
    if (busy()) return
    setBusyWidth(primary?.offsetWidth)
    setBusy(true)
    try {
      await (beforeLaunch() ? fixMismatch : fix).mutateAsync(data().instanceId)
      if (!disposed) close()
    } catch (error) {
      // The fix applied, and the launch stopped to warn about memory: that
      // dialog opens over this one, and its "Play anyway" plays the fixed
      // Java, so this one has nothing left to do.
      if (rspcErrorCode(error) === "INSUFFICIENT_MEMORY") {
        if (!disposed) close()
        return
      }
      // The mutation's error toast tells the user; this keeps the detail.
      console.error("Could not fix the instance's Java:", error)
      if (disposed) return
      setBusy(false)
      primary?.focus()
    }
  }

  // Launches on the Java it's set to, with no second ask.
  const playAnyway = () => {
    if (busy()) return
    launch.mutate({ id: data().instanceId, skipJavaCheck: true })
    close()
  }

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={t("java:_trn_java_issue_title", {
        instance: data().instanceName,
        major: data().fixMajor,
        interpolation: { escapeValue: false }
      })}
      width="w-120"
      height="h-auto"
      compact
    >
      <div class="flex flex-col gap-6">
        <div class="flex flex-col gap-3">
          <p class="text-lightSlate-300 m-0">
            {beforeLaunch()
              ? t("java:_trn_java_issue_set_to", { major: data().usedMajor })
              : t("java:_trn_java_issue_ran_on", { major: data().usedMajor })}
          </p>
          <Show when={install()}>
            {(running) => (
              <div class="flex flex-col gap-1.5">
                <div
                  role="progressbar"
                  aria-label={t("java:_trn_major", {
                    major: running().major
                  })}
                  aria-valuemin={0}
                  aria-valuemax={100}
                  aria-valuenow={percent()}
                  aria-valuetext={progressText()}
                >
                  <Progress value={percent()} />
                </div>
                <span class="text-lightSlate-700 min-h-5 text-sm">
                  {progressText()}
                </span>
              </div>
            )}
          </Show>
        </div>
        {/* Each keeps to one line. A row too long for both wraps upwards:
            the button on top, the link under it. */}
        <DialogFooter>
          <TextLink
            class="whitespace-nowrap"
            disabled={beforeLaunch() && busy()}
            onClick={() => (beforeLaunch() ? playAnyway() : close())}
          >
            {beforeLaunch()
              ? t("java:_trn_java_issue_play_anyway")
              : t("java:_trn_close")}
          </TextLink>
          <Button
            ref={primary}
            class="whitespace-nowrap"
            loading={busy()}
            onClick={play}
            style={
              busy() && busyWidth() !== undefined
                ? { "min-width": `${busyWidth()}px` }
                : undefined
            }
          >
            {t("java:_trn_java_issue_fix", { major: data().fixMajor })}
          </Button>
        </DialogFooter>
      </div>
    </ModalLayout>
  )
}

export default JavaIssueModal
