import { useTransContext } from "@gd/i18n"
import { Spinner, Tooltip, TooltipContent, TooltipTrigger } from "@gd/ui"
import type {
  FEJavaEntry,
  FEJavaGroup,
  FEManagedJavaInstall
} from "@gd/core_module/bindings"
import {
  JSX,
  Match,
  Show,
  Switch,
  createEffect,
  createSignal,
  untrack
} from "solid-js"
import { useModal } from "@/managers/ModalsManager"
import type { DeleteJavaConfirmData } from "@/managers/ModalsManager/modals/Java/DeleteJavaConfirm"
import { rspc } from "@/utils/rspcClient"
import {
  isUpdatingManagedJava,
  updateManagedJava
} from "@/utils/managedJavaUpdates"
import {
  progressLabel,
  progressView,
  vendorLabel
} from "@/managers/ModalsManager/modals/Java/managedJavaCatalog"
import { middleTruncate } from "@/utils/java/javaText"
import {
  advisoryMuted,
  deleteNeedsConfirm,
  deleteTooltip,
  markerText,
  sourceLine
} from "./javaOverviewView"
import AdvisoryPopover from "./AdvisoryPopover"
import { useAdvisoryReview } from "./advisoryReview"
import IconButton, { PATH_TEXT_CLASS, PATH_TOOLTIP_CLASS } from "./IconButton"
import UsedByPopover from "./UsedByPopover"
import { useFormatSize } from "@/utils/java/useFormatSize"

const PATH_MAX = 60

/** A 24px column for one icon, kept when empty so rows line up. */
const IconSlot = (props: { children?: JSX.Element }) => (
  <div class="flex h-6 w-6 shrink-0 items-center justify-center">
    {props.children}
  </div>
)

interface Props {
  entry: FEJavaEntry
  /** Missing for "Other Java versions", whose rows have no marker. */
  group?: FEJavaGroup
  /** Whether `usedBy` is the whole story yet; see `advisoryMuted`. */
  usageComplete: boolean
  /** What tells this Java apart from another of the same name, if one is
   *  listed: Azul's build name for GDLauncher's Javas. */
  hint?: string
  /** Makes this Java the one its group uses. */
  onChoose: () => void
  /** Move focus to the marker once it shows this Java in use. */
  focusMarker: boolean
  onMarkerFocused: () => void
  /** For when the focused control goes away, as an unpinned pin or a deleted
   *  row does. */
  focusHeading: () => void
}

const JavaRow = (props: Props) => {
  const [t] = useTransContext()
  const modalsContext = useModal()
  const review = useAdvisoryReview()
  let row: HTMLDivElement | undefined
  let marker: HTMLButtonElement | undefined
  let deleteButton: HTMLButtonElement | undefined
  let advisoryTrigger: HTMLButtonElement | undefined
  const [advisoryOpen, setAdvisoryOpen] = createSignal(false)

  const unpin = rspc.createMutation(() => ({
    mutationKey: ["java.unpinManagedJava"]
  }))
  const deleteJava = rspc.createMutation(() => ({
    mutationKey: ["java.deleteJavaVersion"]
  }))

  // Asks first when instances use the Java, since they move to Automatic.
  const remove = () => {
    if (deleteJava.isPending || deleteBlocked()) return
    const id = props.entry.id
    if (!deleteNeedsConfirm(props.entry)) {
      deleteJava.mutate(id, { onSuccess: props.focusHeading })
      return
    }

    const data: DeleteJavaConfirmData = {
      vendor: props.entry.vendorName,
      version: props.entry.version,
      source: props.entry.source,
      usedBy: props.entry.usedBy,
      runningGame: () => props.entry.runningGame,
      onConfirm: () => deleteJava.mutateAsync(id),
      onClosed: (deleted) =>
        deleted ? props.focusHeading() : deleteButton?.focus()
    }
    modalsContext?.openModal({ name: "deleteJavaConfirm" }, data)
  }

  const size = useFormatSize()
  const name = () => `${props.entry.vendorName} ${props.entry.version}`
  const inUse = () => props.group?.inUseJavaId === props.entry.id
  // Automatic skips Javas that don't start, so choosing one wouldn't apply.
  const canChoose = () => props.entry.isValid && !props.entry.pendingRemoval
  const source = () => sourceLine(props.entry, t, size)
  const updating = () => isUpdatingManagedJava(props.entry.id)
  // An update replaces this Java, so deleting it meanwhile would race it.
  const deleteBlocked = () => props.entry.runningGame || updating()

  createEffect(() => {
    if (props.focusMarker && inUse() && marker) {
      marker.focus()
      props.onMarkerFocused()
    }
  })

  // The Review landing's own focus is not the user's, so it keeps the
  // highlight.
  let landingFocus = false
  const highlighted = () => review?.isHighlighted(props.entry.id) ?? false
  const unhighlight = () => {
    if (!landingFocus) review?.unhighlight(props.entry.id)
  }

  createEffect(() => {
    const landing = review?.landing()
    if (landing?.javaId !== props.entry.id) return

    untrack(() => {
      review?.landed()
      row?.scrollIntoView({ block: "center", behavior: landing.scroll })
      landingFocus = true
      advisoryTrigger?.focus({ preventScroll: true })
      landingFocus = false
      setAdvisoryOpen(true)
    })
  })

  return (
    <div
      ref={row}
      class="group flex items-center gap-3 rounded-md px-2 py-2"
      classList={{
        "opacity-50": props.entry.pendingRemoval,
        "hover:bg-darkSlate-600": !highlighted(),
        "bg-yellow-500/10 shadow-[inset_2px_0_0_0_rgb(var(--yellow-500)/0.6)]":
          highlighted()
      }}
      // Not pointerenter: that also fires when the landing scrolls the row
      // under a pointer that never moved.
      onPointerMove={unhighlight}
      // Native, so focus inside the row's portalled popovers doesn't count.
      on:focusin={unhighlight}
    >
      <IconSlot>
        <Show when={inUse() || canChoose() ? props.group : undefined}>
          {(group) => {
            const text = () => markerText(group(), name(), inUse(), t)
            // One button for both states, so it keeps focus when the choice
            // applies. A small dot marks the Java in use; the others show an
            // empty one on hover or focus, to choose them.
            return (
              <IconButton
                ref={(el) => (marker = el)}
                label={text().label}
                tooltip={text().tooltip}
                class={
                  inUse()
                    ? "group/marker"
                    : "group/marker opacity-0 focus-visible:opacity-100 group-hover:opacity-100 group-focus-within:opacity-100"
                }
                onClick={() => {
                  if (!inUse()) props.onChoose()
                }}
              >
                <div
                  class="h-[7px] w-[7px] rounded-full"
                  classList={{
                    "bg-primary-300": inUse(),
                    "border border-lightSlate-700 group-hover/marker:border-lightSlate-100 group-focus-visible/marker:border-lightSlate-100":
                      !inUse()
                  }}
                />
              </IconButton>
            )
          }}
        </Show>
      </IconSlot>

      <div class="flex min-w-0 flex-1 flex-col gap-0.5">
        <div class="flex items-center gap-1">
          <span class="text-lightSlate-50 mr-1 whitespace-nowrap">
            {name()}
          </span>
          <Show when={props.hint}>
            {(hint) => (
              <span class="text-lightSlate-700 mr-1 whitespace-nowrap text-xs">
                {hint()}
              </span>
            )}
          </Show>
          <Show when={props.entry.knownIssues.length > 0}>
            <AdvisoryPopover
              entry={props.entry}
              name={name()}
              muted={advisoryMuted(props.entry, props.usageComplete)}
              open={advisoryOpen()}
              onOpenChange={setAdvisoryOpen}
              triggerRef={(el) => (advisoryTrigger = el)}
            />
          </Show>
          <Show when={!props.entry.isValid}>
            <IconButton
              label={t("java:_trn_a11y_does_not_start", { name: name() })}
              tooltip={t("java:_trn_java_does_not_start")}
            >
              <div class="i-hugeicons:alert-02 h-4 w-4 text-yellow-500" />
            </IconButton>
          </Show>
          <Switch>
            <Match when={props.entry.nativity === "rosetta"}>
              <IconButton
                label={t("java:_trn_a11y_rosetta", { name: name() })}
                tooltip={t("java:_trn_runs_through_rosetta")}
              >
                <div class="i-hugeicons:cpu h-4 w-4 text-lightSlate-700" />
              </IconButton>
            </Match>
            <Match when={props.entry.nativity === "x86OnX64"}>
              <IconButton
                label={t("java:_trn_a11y_32_bit", { name: name() })}
                tooltip={t("java:_trn_32_bit_memory_limit")}
              >
                <div class="i-hugeicons:cpu h-4 w-4 text-lightSlate-700" />
              </IconButton>
            </Match>
          </Switch>
          <Show when={props.entry.pinned}>
            <IconButton
              label={t("java:_trn_a11y_unpin", { name: name() })}
              tooltip={t("java:_trn_stays_on_version", {
                version: props.entry.version
              })}
              disabled={props.entry.pendingRemoval}
              class={
                props.entry.pendingRemoval
                  ? "text-lightSlate-700"
                  : "text-lightSlate-700 hover:text-lightSlate-50"
              }
              onClick={() =>
                unpin.mutate(props.entry.id, { onSuccess: props.focusHeading })
              }
            >
              <div class="i-hugeicons:pin h-4 w-4" />
            </IconButton>
          </Show>
        </div>
        <div class="text-lightSlate-700 flex min-w-0 text-xs">
          <span class="shrink-0 whitespace-pre">
            {source().text}
            {source().path ? " · " : ""}
          </span>
          <Show when={source().path}>
            {(path) => (
              // Narrow windows cut even short paths, so the full path is
              // always one hover or focus away.
              <Tooltip>
                <TooltipTrigger
                  as="span"
                  tabIndex={0}
                  class="truncate rounded outline-none focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary-500"
                >
                  {middleTruncate(path(), PATH_MAX)}
                </TooltipTrigger>
                <TooltipContent
                  class={`${PATH_TOOLTIP_CLASS} ${PATH_TEXT_CLASS}`}
                >
                  {path()}
                </TooltipContent>
              </Tooltip>
            )}
          </Show>
        </div>
      </div>

      <div class="flex shrink-0 items-center gap-2">
        <Show when={props.entry.usedBy.length > 0}>
          <UsedByPopover usedBy={props.entry.usedBy} />
        </Show>
        <IconSlot>
          <Show when={props.entry.pendingRemoval ? null : props.entry.updateTo}>
            {(version) => (
              // While it updates, the new build's row below shows the one
              // spinner, with the progress.
              <IconButton
                label={t("java:_trn_a11y_update", {
                  name: name(),
                  version: version()
                })}
                tooltip={t("java:_trn_update_to_version", {
                  version: version()
                })}
                disabled={updating()}
                busy={updating()}
                class={
                  updating()
                    ? "text-darkSlate-300"
                    : "text-lightSlate-700 hover:text-green-500"
                }
                onClick={() => updateManagedJava(props.entry.id, version())}
              >
                <div class="i-hugeicons:download-02 h-5 w-5" />
              </IconButton>
            )}
          </Show>
        </IconSlot>
        <IconSlot>
          <Show when={deleteTooltip(props.entry, t, size)}>
            {(tooltip) => (
              <Switch>
                <Match when={props.entry.pendingRemoval}>
                  {/* Waiting for the game to close, not working. */}
                  <IconButton
                    label={t("java:_trn_a11y_pending_removal", {
                      name: name()
                    })}
                    tooltip={tooltip()}
                    class="text-lightSlate-700"
                  >
                    <div class="i-hugeicons:clock-01 h-5 w-5" />
                  </IconButton>
                </Match>
                <Match when={true}>
                  <IconButton
                    ref={(el) => (deleteButton = el)}
                    label={t(
                      props.entry.source === "custom"
                        ? "java:_trn_a11y_remove"
                        : "java:_trn_a11y_delete",
                      { name: name() }
                    )}
                    tooltip={tooltip()}
                    disabled={deleteBlocked()}
                    busy={deleteJava.isPending}
                    class={
                      deleteBlocked()
                        ? "text-darkSlate-300"
                        : "text-lightSlate-700 hover:text-red-400"
                    }
                    onClick={remove}
                  >
                    <div class="i-hugeicons:delete-02 h-5 w-5" />
                  </IconButton>
                </Match>
              </Switch>
            )}
          </Show>
        </IconSlot>
      </div>
    </div>
  )
}

/** A Java still installing: a spinner with the progress in its tooltip,
 *  over the line it will have once installed, so the row doesn't jump. */
export const JavaInstallRow = (props: { install: FEManagedJavaInstall }) => {
  const [t] = useTransContext()
  const size = useFormatSize()

  const progress = rspc.createQuery(() => ({
    queryKey: ["java.getSetupManagedJavaProgress", props.install.buildId],
    refetchInterval: 500
  }))

  const name = () =>
    `${vendorLabel(props.install.vendor)} ${props.install.version}`
  const installing = () => t("java:_trn_a11y_installing", { name: name() })
  const progressText = () =>
    progressLabel(
      progressView(progress.data ?? props.install.progress),
      t,
      size
    )

  return (
    <div class="flex items-center gap-3 rounded-md px-2 py-2">
      <IconSlot />
      <div class="flex min-w-0 flex-1 flex-col gap-0.5">
        <div class="flex items-center gap-1">
          <span class="text-lightSlate-50 mr-1 whitespace-nowrap">
            {name()}
          </span>
          <IconButton
            label={installing()}
            tooltip={progressText() ?? installing()}
          >
            <Spinner class="h-4 w-4" />
          </IconButton>
        </div>
        <div class="text-lightSlate-700 text-xs">
          {t("java:_trn_installed_by_gdlauncher")}
        </div>
      </div>
    </div>
  )
}

export default JavaRow
