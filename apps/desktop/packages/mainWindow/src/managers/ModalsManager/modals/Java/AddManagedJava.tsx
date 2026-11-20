import { Trans, useTransContext } from "@gd/i18n"
import { ModalProps, useModal } from "@/managers/ModalsManager"
import ModalLayout from "@/managers/ModalsManager/ModalLayout"
import {
  Button,
  Checkbox,
  Collapsable,
  Input,
  Progress,
  Radio,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  Tooltip,
  TooltipContent,
  TooltipTrigger,
  toast
} from "@gd/ui"
import { extractErrorDisplay, rspc } from "@/utils/rspcClient"
import {
  claimInstallError,
  startManagedJavaInstall
} from "@/utils/managedJavaInstalls"
import {
  For,
  Match,
  Show,
  Switch,
  createEffect,
  createMemo,
  createSignal,
  createUniqueId,
  onCleanup,
  untrack
} from "solid-js"
import type { FEManagedJavaBuild, FEVendor } from "@gd/core_module/bindings"
import { javaNeedText, securityFlawText } from "@/utils/java/javaText"
import { useFormatSize } from "@/utils/java/useFormatSize"
import { addJavaFromFile } from "@/pages/Settings/java/addJavaFromFile"
import { TextLink } from "@/components/TextLink"
import {
  buildHints,
  catalogForMajors,
  defaultBuild,
  defaultMajor,
  filterBuilds,
  formatReleaseDate,
  isBuildInstalled,
  majorLabel,
  progressLabel,
  progressView,
  vendorLabel,
  visibleMajors
} from "./managedJavaCatalog"

/** The known-issue icon after a build's version. Its tooltip says what the
 *  issues are. `mouseOnly` in the build list: the build's radio carries the
 *  same text as its description, so the list stays one Tab stop. */
const KnownIssuesIcon = (props: {
  name: string
  issues: string[]
  mouseOnly?: boolean
}) => {
  const [t, { getI18next }] = useTransContext()

  return (
    <Tooltip>
      <TooltipTrigger
        type="button"
        // Spelt as Kobalte's own prop, so this one replaces it.
        tabIndex={props.mouseOnly ? -1 : undefined}
        aria-hidden={props.mouseOnly ? "true" : undefined}
        aria-label={t("java:_trn_a11y_security_flaw", { name: props.name })}
        // In the build list it sits inside the build's radio label, and a
        // click on it must not pick that build.
        onClick={(e: MouseEvent) => e.stopPropagation()}
        class="flex h-5 w-5 shrink-0 items-center justify-center rounded outline-none focus-visible:outline focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-primary-500"
      >
        <div class="i-hugeicons:alert-diamond h-4 w-4 text-yellow-500" />
      </TooltipTrigger>
      <TooltipContent>
        {securityFlawText(props.issues, getI18next().language, t)}
      </TooltipContent>
    </Tooltip>
  )
}

const AddManagedJava = (props: ModalProps) => {
  const [t, { getI18next }] = useTransContext()
  const modalsContext = useModal()
  const requestedMajor: number | undefined =
    typeof props.data?.major === "number" ? props.data.major : undefined
  // Forge 1.16.5 needs Java 8 below 8.0.312: the dialog then offers the
  // newest build under it first.
  const requestedBelow: string | undefined =
    typeof props.data?.below === "string" ? props.data.below : undefined
  // The instance's Minecraft version, to say why.
  const requestedVersion: string | undefined =
    typeof props.data?.minecraftVersion === "string"
      ? props.data.minecraftVersion
      : undefined
  // Told about an install this dialog started once it succeeds, even after
  // the dialog has closed.
  const onInstalled: ((javaId: string) => unknown) | undefined =
    typeof props.data?.onInstalled === "function"
      ? props.data.onInstalled
      : undefined

  const [vendor, setVendor] = createSignal<FEVendor>("azul")
  const [major, setMajor] = createSignal<number>()
  const [showAll, setShowAll] = createSignal(false)
  const [pickedBuildId, setPickedBuildId] = createSignal<string>()
  const [search, setSearch] = createSignal("")
  const [installingId, setInstallingId] = createSignal<string>()
  // The build whose install failed; the reason is in the log.
  const [failedBuildId, setFailedBuildId] = createSignal<string>()
  const [adding, setAdding] = createSignal(false)
  const size = useFormatSize()
  // The build list is one radio group for the keyboard.
  const buildGroup = createUniqueId()

  // The install outlives the dialog; once closed, its outcome is the
  // toast's to report.
  let disposed = false
  onCleanup(() => {
    disposed = true
  })

  const vendors = rspc.createQuery(() => ({
    queryKey: ["java.getManagedVendors"]
  }))
  const catalog = rspc.createQuery(() => ({
    queryKey: ["java.getManagedCatalog", vendor()],
    // The summary line shows the error and a "Try again" link.
    meta: { silentError: true }
  }))
  const minecraftMajorsQuery = rspc.createQuery(() => ({
    queryKey: ["java.getMinecraftJavaMajors"]
  }))
  const javas = rspc.createQuery(() => ({
    queryKey: ["java.getAvailableJavas"]
  }))
  const installs = rspc.createQuery(() => ({
    queryKey: ["java.getManagedJavaInstalls"]
  }))

  const minecraftMajors = () => minecraftMajorsQuery.data ?? []
  // The download server is unreachable: the slot says so, unless "Try
  // again" is under way.
  const catalogFailed = () =>
    catalog.isError && !catalog.data && !catalog.isFetching
  // Whether the last fetch failed, until a catalog arrives. The query alone
  // can't say: a refetch with no data puts it back to pending, error
  // cleared, so "Try again" would empty the select under the user.
  const [unreachable, setUnreachable] = createSignal(false)
  createEffect(() => {
    if (catalog.data) setUnreachable(false)
    else if (catalog.isError) setUnreachable(true)
  })
  // The select lists the Javas Minecraft uses meanwhile, retry included.
  const catalogOrNone = () =>
    catalogForMajors({ data: catalog.data, failed: unreachable() })

  const selectMajor = (next: number | undefined) => {
    setMajor(next)
    setPickedBuildId(undefined)
  }

  // Picks the default major on load, and again when a distribution doesn't
  // have the selected one.
  createEffect(() => {
    const c = catalogOrNone()
    if (c === null || minecraftMajorsQuery.isPending) return

    const current = untrack(major)
    const majors = visibleMajors(c, minecraftMajors(), true)
    if (current !== undefined && majors.includes(current)) return

    const next = defaultMajor(c, minecraftMajors(), requestedMajor)
    selectMajor(next)
    if (
      next !== undefined &&
      !visibleMajors(c, minecraftMajors(), false).includes(next)
    ) {
      setShowAll(true)
    }
  })

  const majorOptions = createMemo(() => {
    const c = catalogOrNone()
    return c === null ? [] : visibleMajors(c, minecraftMajors(), showAll())
  })
  const majorText = (m: number | undefined) => {
    if (m === undefined) return ""
    const lts = catalog.data?.majors.find((x) => x.major === m)?.lts ?? false
    return majorLabel(m, lts, minecraftMajors(), t)
  }

  const builds = () =>
    catalog.data?.majors.find((m) => m.major === major())?.builds ?? []
  const selectedBuild = () =>
    builds().find((b) => b.id === pickedBuildId()) ??
    defaultBuild(
      builds(),
      major() === requestedMajor ? requestedBelow : undefined
    )
  /** Why the build offered first is older than the newest: "Forge 1.16.5
   *  needs Java 8 older than 8.0.312". Only for the capped Java version. */
  const capNote = () =>
    requestedMajor !== undefined &&
    requestedBelow !== undefined &&
    requestedVersion !== undefined &&
    major() === requestedMajor
      ? javaNeedText(requestedVersion, requestedMajor, requestedBelow, t)
      : undefined
  const filteredBuilds = createMemo(() => filterBuilds(builds(), search()))
  // Azul lists some versions twice: its build name tells them apart.
  const hints = createMemo(() => buildHints(builds(), vendor()))

  const installed = () => {
    const build = selectedBuild()
    return (
      build !== undefined &&
      isBuildInstalled(build, Object.values(javas.data ?? {}).flat(), vendor())
    )
  }

  // An install of the selected build may already be running, started from an
  // earlier dialog.
  const progressBuildId = () =>
    installingId() ??
    installs.data?.find((i) => i.buildId === selectedBuild()?.id)?.buildId
  const busy = () => installingId() !== undefined

  const progress = rspc.createQuery(() => ({
    queryKey: ["java.getSetupManagedJavaProgress", progressBuildId() ?? ""],
    enabled: progressBuildId() !== undefined,
    refetchInterval: progressBuildId() !== undefined ? 300 : false
  }))
  /** One reading of the progress, which the bar, its value and the text
   *  all show, so the bar never trails the text. */
  const progressState = createMemo(() => {
    const view = progressView(progress.data ?? "idle")
    return {
      percent:
        view.fraction !== undefined
          ? Math.round(view.fraction * 100)
          : undefined,
      text: progressLabel(view, t, size)
    }
  })

  const failed = () =>
    failedBuildId() !== undefined && failedBuildId() === selectedBuild()?.id

  const formatDate = (iso: string | null) =>
    formatReleaseDate(iso, getI18next().language)

  /** What the summary line shows after the version. */
  const summaryDetails = (build: FEManagedJavaBuild) =>
    [
      formatDate(build.releaseDate),
      build.sizeBytes !== null ? size(build.sizeBytes) : undefined
    ].filter((part) => part !== undefined)

  const issuesIcon = (build: FEManagedJavaBuild, mouseOnly = false) => (
    <Show when={build.knownIssues.length > 0}>
      <KnownIssuesIcon
        name={`${vendorLabel(vendor())} ${build.version}`}
        issues={build.knownIssues}
        mouseOnly={mouseOnly}
      />
    </Show>
  )

  const onShowAllChange = (checked: boolean) => {
    setShowAll(checked)
    const c = catalog.data
    const current = major()
    if (checked || !c || current === undefined) return
    if (!visibleMajors(c, minecraftMajors(), false).includes(current)) {
      selectMajor(defaultMajor(c, minecraftMajors(), requestedMajor))
    }
  }

  const install = async () => {
    const build = selectedBuild()
    if (!build) return

    setFailedBuildId(undefined)
    setInstallingId(build.id)
    let javaId: string
    try {
      javaId = await startManagedJavaInstall(
        { vendor: vendor(), id: build.id },
        { version: build.version, major: build.major }
      )
      if (!disposed) modalsContext?.closeModal()
    } catch {
      // `startManagedJavaInstall` logged the reason.
      if (disposed) return
      claimInstallError(build.id)
      setFailedBuildId(build.id)
      return
    } finally {
      setInstallingId(undefined)
    }

    // Outside the install's `try`: what the opener does next is not the
    // install, and its failure must not read as one.
    await tellOpener(javaId)
  }

  /** Tells the opener a Java was added, as an instance's "Download Java
   *  N" waits to check its Java still fits. */
  const tellOpener = async (javaId: string) => {
    try {
      await onInstalled?.(javaId)
    } catch (error) {
      console.error(
        "Java added, but what was to follow failed:",
        extractErrorDisplay(error)
      )
    }
  }

  // Offline, a Java already on the computer still gets added: the same flow
  // as Settings › Java's "Add from a file".
  const addFromFile = async () => {
    if (adding()) return
    setAdding(true)
    try {
      const added = await addJavaFromFile(t)
      if (!added) return
      toast.success(
        t("java:_trn_added_java", {
          vendor: added.vendorName,
          version: added.version
        })
      )
      if (!disposed) modalsContext?.closeModal()
      await tellOpener(added.id)
    } finally {
      setAdding(false)
    }
  }

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      width="w-120"
      height="h-auto"
      compact
    >
      {/* Small gaps: Advanced's toggle brings its own space above and
          below. The top padding keeps the select's focus outline inside
          the dialog's clipped content area. */}
      <div class="flex flex-col gap-2 pt-1">
        <div class="grid grid-cols-[1fr_18rem] items-center gap-x-4 gap-y-2">
          <h5 class="text-lightSlate-400 m-0 text-sm">
            <Trans key="java:_trn_java_version_label" />
          </h5>
          <Select
            value={major()}
            options={majorOptions()}
            disabled={busy() || majorOptions().length === 0}
            onChange={(m) => {
              if (m !== null) selectMajor(m)
            }}
            itemComponent={(itemProps) => (
              <SelectItem item={itemProps.item}>
                {majorText(itemProps.item.rawValue)}
              </SelectItem>
            )}
          >
            {/* The dialog starts here, or once the catalog has loaded. */}
            <SelectTrigger class="w-full" autofocus>
              <SelectValue<number>
                class="min-w-0 truncate"
                title={majorText(major())}
              >
                {(state) => majorText(state.selectedOption())}
              </SelectValue>
            </SelectTrigger>
            <SelectContent />
          </Select>
          {/* One slot under the select for what's going on: the
              download server, the install's progress or failure, or
              the build that installs. */}
          <div class="text-lightSlate-700 col-start-2 flex min-h-5 flex-wrap items-center gap-x-1.5 gap-y-1 text-sm">
            <Switch>
              <Match when={catalogFailed()}>
                <span class="basis-full text-red-300">
                  <Trans key="java:_trn_catalog_failed" />
                </span>
                <TextLink onClick={() => catalog.refetch()}>
                  <Trans key="java:_trn_try_again" />
                </TextLink>
                <span class="text-darkSlate-400">·</span>
                <TextLink
                  tone="muted"
                  disabled={adding()}
                  onClick={addFromFile}
                >
                  <Trans key="java:_trn_add_from_file" />
                </TextLink>
              </Match>
              <Match when={progressBuildId()}>
                <div class="flex w-full flex-col gap-1.5">
                  <div
                    role="progressbar"
                    aria-label={selectedBuild()?.version}
                    aria-valuemin={0}
                    aria-valuemax={100}
                    aria-valuenow={progressState().percent}
                    aria-valuetext={progressState().text}
                  >
                    <Progress
                      value={progressState().percent}
                      // Moves with the text, rather than easing after it.
                      barStyle={{ transition: "none" }}
                    />
                  </div>
                  <span class="min-h-5">{progressState().text}</span>
                </div>
              </Match>
              <Match when={failed() ? selectedBuild() : undefined}>
                {(build) => (
                  <p
                    role="alert"
                    class="m-0 w-full break-words text-sm text-red-300"
                  >
                    {t("java:_trn_download_failed", {
                      major: build().major
                    })}
                  </p>
                )}
              </Match>
              <Match when={selectedBuild()}>
                {(build) => (
                  <>
                    <span>{build().version}</span>
                    {issuesIcon(build())}
                    <For each={summaryDetails(build())}>
                      {(part) => (
                        <>
                          <span class="text-darkSlate-400">·</span>
                          <span>{part}</span>
                        </>
                      )}
                    </For>
                    <Show when={capNote()}>
                      {(note) => <span class="basis-full">{note()}</span>}
                    </Show>
                  </>
                )}
              </Match>
            </Switch>
          </div>
        </div>
        <Collapsable
          title={<Trans key="instances:_trn_instance_share.advanced" />}
          defaultOpened={false}
          noPadding
          size="small"
        >
          <div class="flex flex-col gap-3 pb-2 pt-1">
            <Show when={(vendors.data?.length ?? 0) > 1}>
              <div class="grid grid-cols-[1fr_18rem] items-center gap-x-4">
                <h5 class="text-lightSlate-400 m-0 text-sm">
                  <Trans key="java:_trn_distribution" />
                </h5>
                <Select
                  value={vendor()}
                  options={vendors.data ?? []}
                  disabled={busy()}
                  onChange={(v) => {
                    if (v === null) return
                    setVendor(v)
                    setPickedBuildId(undefined)
                  }}
                  itemComponent={(itemProps) => (
                    <SelectItem item={itemProps.item}>
                      {vendorLabel(itemProps.item.rawValue)}
                    </SelectItem>
                  )}
                >
                  <SelectTrigger class="w-full">
                    <SelectValue<FEVendor>>
                      {(state) => {
                        const v = state.selectedOption()
                        return v ? vendorLabel(v) : ""
                      }}
                    </SelectValue>
                  </SelectTrigger>
                  <SelectContent />
                </Select>
              </div>
            </Show>
            <Checkbox
              checked={showAll()}
              disabled={busy()}
              onChange={onShowAllChange}
            >
              <span class="text-lightSlate-300 text-sm">
                <Trans key="java:_trn_show_all_java_versions" />
              </span>
            </Checkbox>
            <Input
              value={search()}
              onInput={(e) => setSearch(e.currentTarget.value)}
              placeholder={t("java:_trn_search_builds")}
              icon={<div class="i-hugeicons:search-01 h-4 w-4" />}
              inputColor="bg-darkSlate-800"
              disabled={busy()}
              class="w-full"
            />
            <div
              class="flex max-h-60 flex-col gap-0.5 overflow-y-auto"
              classList={{ "pointer-events-none opacity-50": busy() }}
              // Nor from the keyboard while a build installs.
              inert={busy()}
            >
              <For
                each={filteredBuilds()}
                fallback={
                  <Show when={builds().length > 0}>
                    <p class="text-lightSlate-700 m-0 p-1.5 text-sm">
                      <Trans key="java:_trn_no_builds_match" />
                    </p>
                  </Show>
                }
              >
                {(build) => {
                  // The icon's tooltip text, for the keyboard and screen
                  // readers: hidden, so it stays out of the radio's name.
                  const issuesId = createUniqueId()
                  const hasIssues = () => build.knownIssues.length > 0
                  return (
                    <Radio
                      name={buildGroup}
                      value={build.id}
                      checked={build.id === selectedBuild()?.id}
                      aria-describedby={hasIssues() ? issuesId : undefined}
                      onChange={() => setPickedBuildId(build.id)}
                    >
                      <span class="flex items-center justify-between gap-2 text-sm">
                        <span class="flex items-center gap-1.5">
                          <span class="text-lightSlate-100">
                            {build.version}
                          </span>
                          <Show when={hints().get(build.id)}>
                            {(hint) => (
                              <span class="text-lightSlate-700 text-xs">
                                {hint()}
                              </span>
                            )}
                          </Show>
                          {issuesIcon(build, true)}
                          <Show when={hasIssues()}>
                            <span id={issuesId} hidden>
                              {securityFlawText(
                                build.knownIssues,
                                getI18next().language,
                                t
                              )}
                            </span>
                          </Show>
                        </span>
                        <span class="text-lightSlate-700">
                          {formatDate(build.releaseDate)}
                        </span>
                      </span>
                    </Radio>
                  )
                }}
              </For>
            </div>
          </div>
        </Collapsable>
        <div class="flex justify-end">
          <Button
            loading={progressBuildId() !== undefined}
            disabled={!selectedBuild() || installed()}
            onClick={install}
          >
            <Switch fallback={<Trans key="java:_trn_install" />}>
              <Match when={installed()}>
                <Trans key="java:_trn_installed" />
              </Match>
              <Match when={failed()}>
                <Trans key="java:_trn_try_again" />
              </Match>
            </Switch>
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default AddManagedJava
