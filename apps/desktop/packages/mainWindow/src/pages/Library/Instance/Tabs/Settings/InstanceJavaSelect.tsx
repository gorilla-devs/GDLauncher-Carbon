import { useTransContext } from "@gd/i18n"
import {
  Button,
  Popover,
  PopoverContent,
  PopoverTrigger,
  Select,
  SelectContent,
  SelectItem,
  SelectSection,
  SelectTrigger,
  SelectValue,
  Tooltip,
  TooltipContent,
  TooltipTrigger
} from "@gd/ui"
import type {
  FEJavaEntry,
  FEJavaOverride,
  InstanceDetails
} from "@gd/core_module/bindings"
import { For, JSX, Show, createMemo, createSignal } from "solid-js"
import { useModal } from "@/managers/ModalsManager"
import { rspc } from "@/utils/rspcClient"
import { createJavaOverviewQuery } from "@/pages/Settings/settings.java.data"
import { addJavaFromFile } from "@/pages/Settings/java/addJavaFromFile"
import {
  ICON_BUTTON_CLASS,
  PATH_TEXT_CLASS,
  PATH_TOOLTIP_CLASS
} from "@/pages/Settings/java/IconButton"
import { javaChoices } from "@/pages/Settings/java/javaOverviewView"
import {
  javaNeedText,
  labelHints,
  securityFlawText
} from "@/utils/java/javaText"
import {
  automaticIfStillMismatched,
  buildJavaOptions,
  currentOption,
  isAction,
  isSystemProfile,
  javaNeed,
  misfitNote,
  optionKey,
  optionLabel,
  overrideFor,
  type InstanceJavaClient,
  type JavaOption,
  type JavaOptionSection
} from "./javaSelectOptions"

interface Props {
  details: InstanceDetails
  /** Stores the instance's Java override; `null` is Automatic. */
  setOverride: (javaOverride: FEJavaOverride | null) => void
}

/** Reads and writes an instance through the plain client, not this page's
 *  queries: after an install the user may be on another page, or none. The
 *  core then refreshes the instance's details wherever they're shown. */
const instanceJavaClient: InstanceJavaClient = {
  details: (instanceId) =>
    rspc.useContext().client.query(["instance.getInstanceDetails", instanceId]),
  setAutomatic: (instanceId) =>
    rspc
      .useContext()
      .client.mutation([
        "instance.updateInstance",
        { instance: instanceId, javaOverride: { Set: null } }
      ])
}

/** A line of muted text next to an option's name. */
const Muted = (props: { children: JSX.Element }) => (
  <span class="text-lightSlate-700 min-w-0 truncate text-xs">
    {props.children}
  </span>
)

/** The Java an instance runs on, in one dropdown: Automatic, a Java, a
 *  profile, or a file. A Java that doesn't suit the instance's Minecraft
 *  version gets a yellow icon whose popover offers a Java that does. */
const InstanceJavaSelect = (props: Props) => {
  const [t, { getI18next }] = useTransContext()
  const modalsContext = useModal()
  let trigger: HTMLButtonElement | undefined
  const [adding, setAdding] = createSignal(false)
  const [mismatchOpen, setMismatchOpen] = createSignal(false)
  const [listOpen, setListOpen] = createSignal(false)
  // After "Download Java N" or "Choose a file" the list hands focus back to
  // the trigger while a dialog is up, so its tooltip waits for a blur.
  const [afterAction, setAfterAction] = createSignal(false)

  const overview = createJavaOverviewQuery()
  const profiles = rspc.createQuery(() => ({
    queryKey: ["java.getJavaProfiles"]
  }))

  const entries = () => (overview.data ? javaChoices(overview.data) : [])
  const hints = createMemo(() => labelHints(entries()))
  const mcVersion = () => props.details.version ?? ""
  const need = createMemo(() => javaNeed(props.details))
  const resolved = () => props.details.resolvedJava
  // What Automatic downloads at launch, while no Java that fits is there.
  const download = () => {
    const major = props.details.downloadMajor
    return major === null ? null : { major, below: props.details.downloadBelow }
  }

  const sections = createMemo(
    () =>
      buildJavaOptions(
        need(),
        mcVersion(),
        entries(),
        profiles.data ?? [],
        download(),
        t
      ).sections
  )
  const current = createMemo(() =>
    currentOption(
      props.details.javaOverride,
      props.details.requiredJavaProfile,
      entries(),
      profiles.data
    )
  )

  const needText = () => {
    const n = need()
    return n === null
      ? ""
      : javaNeedText(mcVersion(), n.major, n.below, t, n.floor)
  }

  /** What Automatic gives the instance, whatever it's set to: "Java 21 ·
   *  Azul Zulu 21.0.13.1", or that it downloads at launch. */
  const automaticDetail = () => {
    const java = props.details.automaticJava
    if (typeof java?.major !== "number") return undefined
    if (java.downloadsAtLaunch) {
      return t("java:_trn_major_downloads_at_launch", { major: java.major })
    }
    if (java.vendorName === null || java.version === null) return undefined
    return t("java:_trn_major_vendor_version", {
      major: java.major,
      vendor: java.vendorName,
      version: java.version
    })
  }

  const javaName = (e: FEJavaEntry) => `${e.vendorName} ${e.version}`.trim()

  /** Where a Java comes from, when it isn't GDLauncher's own. */
  const source = (e: FEJavaEntry) => {
    if (e.id === "") return undefined
    switch (e.source) {
      case "local":
        return t("java:_trn_on_this_computer")
      case "custom":
        return t("java:_trn_added_by_you")
      default:
        return undefined
    }
  }

  /** The trigger: its name, muted detail, tooltip lines and path, and
   *  whether its Java doesn't start. */
  const shown = createMemo(
    (): {
      name: string
      detail?: string
      tooltip: string[]
      path?: string
      broken?: boolean
    } => {
      const option = current()
      switch (option.kind) {
        case "automatic": {
          const detail = automaticDetail()
          const name = t("java:_trn_automatic")
          return {
            name,
            detail,
            tooltip: [detail ? `${name} · ${detail}` : name],
            path: resolved()?.path ?? undefined
          }
        }
        case "java": {
          // A path no list has hasn't been checked, so it isn't known broken.
          const broken = option.entry.id !== "" && !option.entry.isValid
          return {
            name: javaName(option.entry),
            tooltip: broken ? [t("java:_trn_java_does_not_start")] : [],
            path: option.entry.path,
            broken
          }
        }
        case "profile": {
          const java = resolved()
          const major = option.major ?? java?.major ?? undefined
          const javaText =
            major === undefined ? "" : t("java:_trn_major", { major })
          // The launcher's own profiles have codenames: name the Java instead.
          if (isSystemProfile(option.name)) {
            const name = javaText || t("java:_trn_java")
            const detail =
              java?.vendorName && java.version
                ? `${java.vendorName} ${java.version}`
                : undefined
            return {
              name,
              detail,
              tooltip: [detail ? `${name} · ${detail}` : name]
            }
          }
          return {
            name: option.name,
            detail: javaText || undefined,
            tooltip: [javaText ? `${option.name} · ${javaText}` : option.name]
          }
        }
        default:
          return { name: "", tooltip: [] }
      }
    }
  )

  /** The trigger's name, then its muted detail at the end. */
  const TriggerContent = () => (
    <>
      <span class="flex min-w-0 items-center gap-1.5">
        <Show when={shown().broken}>
          <span
            role="img"
            aria-label={t("java:_trn_java_does_not_start")}
            class="i-hugeicons:alert-02 h-4 w-4 shrink-0 text-yellow-500"
          />
        </Show>
        <span class="text-lightSlate-50 min-w-0 truncate">{shown().name}</span>
      </span>
      <Show when={shown().detail}>
        {(detail) => (
          <span class="text-lightSlate-700 min-w-0 shrink-[3] truncate text-xs">
            {detail()}
          </span>
        )}
      </Show>
    </>
  )

  const chooseFile = async () => {
    if (adding()) return
    setAdding(true)
    try {
      const added = await addJavaFromFile(t)
      if (added) props.setOverride({ Path: added.path })
    } finally {
      setAdding(false)
    }
  }

  const choose = (option: JavaOption | null) => {
    // Kobalte also reports the current value, and clears it on a second pick.
    if (!option || optionKey(option) === optionKey(current())) return

    if (isAction(option)) {
      setAfterAction(true)
      if (option.kind === "download") {
        // This instance, whatever page the user is on once it's installed.
        const instanceId = props.details.id
        modalsContext?.openModal(
          { name: "addManagedJava" },
          {
            major: option.major,
            // Forge 1.16.5: the newest build it can run on comes first.
            below: option.below,
            minecraftVersion: mcVersion(),
            // Automatic picks the new build up. An instance set to a Java
            // that doesn't fit goes back to Automatic to use it; one set to
            // a Java that fits keeps it.
            onInstalled: () =>
              automaticIfStillMismatched(instanceId, instanceJavaClient)
          }
        )
      } else {
        void chooseFile()
      }
      return
    }
    props.setOverride(overrideFor(option))
  }

  const fixMismatch = rspc.createMutation(() => ({
    mutationKey: ["instance.fixJavaMismatch"]
  }))
  // The Java the fix moves the instance to: Automatic's when that fits,
  // otherwise one that does, as the dialog before a launch would.
  const fixMajor = () =>
    props.details.javaMismatchFixMajor ??
    props.details.automaticJava?.major ??
    need()?.major

  const useFittingJava = () => {
    if (fixMismatch.isPending) return
    const close = () => {
      setMismatchOpen(false)
      // The icon goes away with the mismatch, so focus moves to the select.
      requestAnimationFrame(() => trigger?.focus())
    }
    if (props.details.javaMismatchFixMajor === null) {
      props.setOverride(null)
      close()
      return
    }
    fixMismatch.mutate(props.details.id, { onSuccess: close })
  }

  const renderOption = (option: JavaOption) => {
    switch (option.kind) {
      case "automatic":
        return (
          <span class="flex min-w-0 items-baseline gap-2">
            <span class="shrink-0">{t("java:_trn_automatic")}</span>
            <Show when={automaticDetail()}>
              {(detail) => <Muted>{detail()}</Muted>}
            </Show>
          </span>
        )
      case "java": {
        const e = option.entry
        // A Java that doesn't fit is dimmed, and its description says why.
        return (
          <span class="flex min-w-0 items-center gap-2">
            <span class="shrink-0">{optionLabel(option, need(), t)}</span>
            <Show when={source(e)}>{(text) => <Muted>{text()}</Muted>}</Show>
            <Show when={hints().get(e.id)}>
              {(hint) => <Muted>{hint()}</Muted>}
            </Show>
            <Show when={e.knownIssues.length > 0}>
              <span class="i-hugeicons:alert-diamond h-4 w-4 shrink-0 text-yellow-500" />
            </Show>
            <Show when={!e.isValid}>
              <span class="i-hugeicons:alert-02 h-4 w-4 shrink-0 text-yellow-500" />
            </Show>
          </span>
        )
      }
      case "download":
        return t("java:_trn_download_java_major", { major: option.major })
      case "profile":
        return (
          <span class="flex min-w-0 items-baseline gap-2">
            <span class="min-w-0 truncate">{option.name}</span>
            <Show when={option.major}>
              {(major) => (
                <Muted>{t("java:_trn_major", { major: major() })}</Muted>
              )}
            </Show>
          </span>
        )
      case "file":
        return (
          <span class="flex items-center gap-2">
            <span class="i-hugeicons:folder-open h-4 w-4 shrink-0" />
            {t("java:_trn_choose_a_file")}
          </span>
        )
      default: {
        const unhandled: never = option
        return unhandled
      }
    }
  }

  /** A profile's Java, from the list, when it starts: one that doesn't
   *  leaves the profile on Automatic. */
  const profileJava = (name: string) => {
    const javaId = profiles.data?.find((p) => p.name === name)?.javaId
    return entries().find((e) => e.id === javaId && e.isValid)
  }

  /** Whether an option is dimmed: a Java or profile that doesn't fit. */
  const dimmed = (option: JavaOption) =>
    (option.kind === "java" && !option.fits) ||
    (option.kind === "profile" && option.fits === false)

  /** An option's accessible extra, for what its icons say. */
  const optionDescription = (option: JavaOption) => {
    if (option.kind === "profile") {
      const java = option.fits === false ? profileJava(option.name) : undefined
      return java && misfitNote(java, need(), mcVersion(), t)
    }
    if (option.kind !== "java") return undefined
    const notes = [
      !option.fits && misfitNote(option.entry, need(), mcVersion(), t),
      option.entry.knownIssues.length > 0 &&
        securityFlawText(
          option.entry.knownIssues.map((issue) => issue.name),
          getI18next().language,
          t
        ),
      !option.entry.isValid && t("java:_trn_java_does_not_start")
    ].filter((note): note is string => !!note)
    return notes.length > 0 ? notes.join(". ") : undefined
  }

  return (
    <div class="flex items-center gap-2">
      <Show when={props.details.javaOverrideMismatch && needText()}>
        <Popover
          open={mismatchOpen()}
          onOpenChange={setMismatchOpen}
          placement="bottom-end"
        >
          {/* The popover says the same, and a tooltip left open under the
              pointer would take the Esc meant for the popover. */}
          <Tooltip open={mismatchOpen() ? false : undefined}>
            <PopoverTrigger
              as={TooltipTrigger}
              aria-label={t("java:_trn_a11y_wrong_java", {
                name: props.details.name
              })}
              class={ICON_BUTTON_CLASS}
            >
              <div class="i-hugeicons:alert-02 h-5 w-5 text-yellow-500" />
            </PopoverTrigger>
            <TooltipContent>{needText()}</TooltipContent>
          </Tooltip>
          <PopoverContent
            aria-label={t("java:_trn_a11y_wrong_java", {
              name: props.details.name
            })}
            class="w-80"
            // One sentence and one button: Esc or a click outside closes it.
            hideCloseButton
          >
            <div class="flex flex-col gap-3">
              <p class="text-lightSlate-50 m-0 text-sm">{needText()}</p>
              <div class="flex justify-end">
                <Button
                  type="secondary"
                  size="small"
                  loading={fixMismatch.isPending}
                  onClick={useFittingJava}
                >
                  {t("java:_trn_use_java_major", { major: fixMajor() ?? "" })}
                </Button>
              </div>
            </div>
          </PopoverContent>
        </Popover>
      </Show>
      <Select<JavaOption, JavaOptionSection>
        value={current()}
        options={sections()}
        optionValue={optionKey}
        // Typeahead matches what each option shows.
        optionTextValue={(option) => optionLabel(option, need(), t)}
        optionDisabled={(option) =>
          option.kind === "java" &&
          (!option.entry.isValid || option.entry.pendingRemoval)
        }
        optionGroupChildren="options"
        optionKey={(item) =>
          "options" in item
            ? `${item.header ?? ""}|${item.options.map(optionKey).join(",")}`
            : optionKey(item)
        }
        onChange={choose}
        onOpenChange={setListOpen}
        // A path or profile the list doesn't have still shows.
        placeholder={<TriggerContent />}
        placement="bottom-end"
        sameWidth={false}
        // A long list stays in the window and scrolls, Automatic included.
        fitViewport
        itemComponent={(itemProps) => {
          const option = itemProps.item.rawValue
          return (
            <SelectItem
              item={itemProps.item}
              aria-description={optionDescription(option)}
              class={dimmed(option) ? "opacity-60" : ""}
            >
              {renderOption(option)}
            </SelectItem>
          )
        }}
        sectionComponent={(sectionProps) => (
          <Show
            when={sectionProps.section.rawValue.header}
            fallback={
              // A rule above "Choose a file", as it's not a Java.
              <Show
                when={sectionProps.section.rawValue.options[0]?.kind === "file"}
              >
                <SelectSection class="bg-darkSlate-600 mx-1 my-1 h-px" />
              </Show>
            }
          >
            {(header) => (
              <SelectSection class="text-lightSlate-700 px-2 pb-1 pt-2 text-xs font-medium uppercase tracking-wide">
                {header()}
              </SelectSection>
            )}
          </Show>
        )}
      >
        {/* Not over the open list, nor over the dialog an action opened. */}
        <Tooltip
          placement="top-end"
          open={listOpen() || afterAction() ? false : undefined}
        >
          <SelectTrigger
            as={TooltipTrigger}
            ref={trigger}
            onBlur={() => setAfterAction(false)}
            aria-label={t("java:_trn_java")}
            class="w-auto max-w-80 gap-3"
          >
            <SelectValue<JavaOption> class="flex min-w-0 flex-1 items-baseline justify-between gap-3">
              {() => <TriggerContent />}
            </SelectValue>
          </SelectTrigger>
          <TooltipContent class={PATH_TOOLTIP_CLASS}>
            <For each={shown().tooltip.filter((line) => line !== "")}>
              {(line) => <div>{line}</div>}
            </For>
            <Show when={shown().path}>
              {(path) => <div class={PATH_TEXT_CLASS}>{path()}</div>}
            </Show>
          </TooltipContent>
        </Tooltip>
        {/* As tall as the window lets it be, less its border, then it
            scrolls. */}
        <SelectContent class="min-w-80 max-w-[28rem] [&>div]:max-h-[calc(var(--kb-popper-content-available-height)-2px)]" />
      </Select>
    </div>
  )
}

export default InstanceJavaSelect
