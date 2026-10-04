import { createEffect, createMemo, createSignal, For, Show } from "solid-js"
import { keepPreviousData } from "@tanstack/solid-query"
import { rspc } from "@/utils/rspcClient"
import { ModalProps, useModal } from "../.."
import ModalLayout from "../../ModalLayout"
import { freezeSelection, reconcileSelections } from "./selections"
import { Button, Checkbox, Spinner, toast } from "@gd/ui"
import { Trans, useTransContext, type TypedTFunction } from "@gd/i18n"
import { useLocation } from "@solidjs/router"
import { useGDNavigate } from "@/managers/NavigationManager"
import { formatBytes } from "@/utils/formatBytes"
import {
  CleanupSelection,
  FEUntrackedFile,
  FEOriginVerdict,
  FERepairReason
} from "@gd/core_module/bindings"

/** Translated label for one `FERepairEntry.reason`, shown in the raw
 *  expandable file list. Every arm mirrors the exact `(PlanAction,
 *  PlanReason)` pairing `apply_plan::decide_repair` / `decide_dropped`
 *  actually produce (see `apply_plan.rs`) — e.g. `DroppedButModified` and
 *  `ModifiedByUser` both resolve to `Keep`, not a destructive action, so
 *  their labels say "kept", not "removed"/"reset". `DisabledReplaceResumed`
 *  is the one exception: it's `decide_version_change`-only (an interrupted
 *  disabled Replace's finishing Delete), so this repair preview — which
 *  only ever runs `decide_repair` — can never actually produce it; still
 *  given a real label rather than falling through, since `FERepairReason`
 *  is one enum shared with the rest of `PlanReason`. */
function repairReasonLabel(t: TypedTFunction, reason: FERepairReason): string {
  switch (reason) {
    case "PackUpdate":
      return t("instances:_trn_repair_reason_pack_update")
    case "Unchanged":
      return t("instances:_trn_repair_reason_unchanged")
    case "ModifiedByUser":
      return t("instances:_trn_repair_reason_modified_kept")
    case "DeletedByUser":
      return t("instances:_trn_repair_reason_deleted_kept")
    case "DisabledByUser":
      return t("instances:_trn_repair_reason_disabled_kept")
    case "InSaveFolder":
      return t("instances:_trn_repair_reason_save_kept")
    case "PackDropped":
      return t("instances:_trn_repair_reason_pack_dropped")
    case "DroppedButModified":
      return t("instances:_trn_repair_reason_dropped_modified_kept")
    case "PreservedExisting":
      return t("instances:_trn_repair_reason_preserved")
    case "RepairOverwrote":
      return t("instances:_trn_repair_reason_overwrote")
    case "RepairRestored":
      return t("instances:_trn_repair_reason_restored")
    case "ReEnabled":
      return t("instances:_trn_repair_reason_reenabled")
    case "CaseAliasedByTarget":
      return t("instances:_trn_repair_reason_case_aliased")
    case "DisabledReplaceResumed":
      return t("instances:_trn_repair_reason_disabled_replace_resumed")
    default:
      return reason
  }
}

/** Display text for an untracked file's origin check verdict, or `null`
 *  before any check has run for this exact path (distinct from the
 *  post-check `"Unknown"` verdict, which does render). */
function originVerdictText(
  t: TypedTFunction,
  verdict: FEOriginVerdict | null
): string | null {
  if (verdict === null) return null
  if (verdict === "CurrentVersion") {
    return t("instances:_trn_repair_origin_current")
  }
  if (verdict === "Incomplete") {
    return t("instances:_trn_repair_origin_incomplete")
  }
  if (verdict === "Unknown") {
    return t("instances:_trn_repair_origin_unknown")
  }
  return t("instances:_trn_repair_origin_shipped_in", {
    version: verdict.ShippedIn.version_name
  })
}

const RepairModpack = (props: ModalProps) => {
  const [t] = useTransContext()
  const modalsContext = useModal()
  const location = useLocation()
  const navigate = useGDNavigate()

  const isServer = () => !!props.data?.isServer
  const instanceId = () => props.data?.id as number

  const [reEnable, setReEnable] = createSignal(false)
  const [ticked, setTicked] = createSignal<Map<string, CleanupSelection>>(
    new Map(),
    {
      equals: false
    }
  )
  const [expanded, setExpanded] = createSignal(false)
  const [originTaskId, setOriginTaskId] = createSignal<number | null>(null)

  const toggleTicked = (file: FEUntrackedFile) => {
    const path = file.path
    setTicked((prev) => {
      const next = new Map(prev)
      if (next.has(path)) {
        next.delete(path)
      } else {
        if (file.selection) next.set(path, freezeSelection(file.selection))
      }
      return next
    })
  }

  // If the user triggered a repair from inside the detail page of the
  // instance/server being repaired, kick them back to the library —
  // the page is about to wipe its `.setup` and re-run the install pipeline,
  // so it'd just show a broken / loading state until the task finishes.
  const navigateAwayIfInsideDetail = () => {
    const pathname = location.pathname
    const id = props.data?.id
    if (id == null) return
    const prefix = isServer() ? `/library/server/${id}` : `/library/${id}`
    if (pathname === prefix || pathname.startsWith(`${prefix}/`)) {
      navigate.navigate(isServer() ? "/library?mode=servers" : "/library")
    }
  }

  // An unfinished install or update never blocks Repair: Confirm discards
  // it (the core keeps it as `.setup-cancelled-*`) and reinstalls the pack.
  // The core only discards the setup this status showed (`expected_pending`).
  const pendingSetup = rspc.createQuery(() => ({
    queryKey: ["instance.getPendingSetup", instanceId()],
    enabled: !isServer(),
    refetchInterval: 2000
  }))
  const pending = () => !isServer() && !!pendingSetup.data?.pending
  // The instance is being installed, launched, run or deleted: the core
  // refuses Repair (and Cancel or Resume) until it stops, so the dialog says
  // so and offers nothing to click.
  const busy = () => !isServer() && !!pendingSetup.data?.busy
  // An install or update still running: nothing to discard or preview yet.
  const installing = () => pending() && busy()
  // The setup may have changed installed files, so the installed record
  // can't predict what Repair changes: no preview, no file selection.
  const previewWithheld = () =>
    !isServer() && !!pendingSetup.data?.preview_withheld
  const cancelPending = rspc.createMutation(() => ({
    mutationKey: ["instance.cancelPendingSetup"],
    onSuccess: () => {
      toast.success(t("instances:_trn_repair_pending_cancelled"))
      modalsContext?.closeModal()
    },
    meta: LOCAL_ERROR_TOAST,
    onError: (error) => {
      toastMutationError(
        t("instances:_trn_repair_pending_cancel_failed"),
        error
      )
      void pendingSetup.refetch()
    }
  }))
  // Play's own preparation (with a deep check), without launching.
  const resumeSetup = rspc.createMutation(() => ({
    mutationKey: ["instance.prepareInstance"],
    onSuccess: () => {
      toast.success(t("instances:_trn_repair_pending_resumed"))
      modalsContext?.closeModal()
    },
    meta: LOCAL_ERROR_TOAST,
    onError: (error) => {
      toastMutationError(
        t("instances:_trn_repair_pending_resume_failed"),
        error
      )
      void pendingSetup.refetch()
    }
  }))

  // Keyed on the instance alone: the backend returns both
  // `re_enable_disabled` outcomes (`with_re_enable`/`without_re_enable`) in
  // one response, computed over a single disk scan — see `RepairPreview`'s
  // own doc (`managers/instance/modpack/mod.rs`). Toggling the checkbox
  // below is therefore a pure client-side selection between the two
  // already-fetched variants (`selectedVariant` below), never a refetch.
  const preview = rspc.createQuery(() => ({
    queryKey: ["instance.getRepairPreview", { instance: instanceId() }],
    enabled:
      !isServer() &&
      pendingSetup.isSuccess &&
      !pendingSetup.data?.preview_withheld &&
      !installing(),
    // Keeps the previous response visible (as `isPlaceholderData`) during a
    // background refetch — e.g. right after a `checkPackOrigin` run
    // completes and invalidates this query to surface fresh untracked-file
    // verdicts — instead of clearing `data` and re-showing the loading
    // fallback below for a query that already has something to show.
    placeholderData: keepPreviousData
  }))

  // The counts/entries half of `preview.data` the re-enable checkbox
  // currently selects; `has_packinfo`/`untracked`/`duplicates` don't depend
  // on `re_enable_disabled` and are read directly off `preview.data`.
  const selectedVariant = createMemo(() =>
    reEnable() ? preview.data?.with_re_enable : preview.data?.without_re_enable
  )

  // Only Modrinth packs can be origin-checked (see
  // `InstanceManager::check_pack_origin`'s own doc comment) — the button
  // that fires it must stay hidden for a CurseForge pack rather than let the
  // user hit the backend's refusal. Nothing in `props.data` carries the
  // pack's platform (none of the six call sites pass one), so it's read off
  // the instance's own details instead of widening every openModal payload
  // for a single button's visibility.
  const instanceDetails = rspc.createQuery(() => ({
    queryKey: ["instance.getInstanceDetails", instanceId()],
    enabled: !isServer()
  }))
  const isModrinthPack = createMemo(
    () => instanceDetails.data?.modpack?.modpack.type === "modrinth"
  )
  // The configured pack's version name, shown for the version Repair
  // installs when that is the configured one.
  const modpackInfo = rspc.createQuery(() => ({
    queryKey: ["instance.getModpackInfo", instanceId()],
    enabled: pending()
  }))
  // Set when Repair installs another version than the configured one: the
  // one an interrupted change was installing. Named by its platform, as the
  // configured one is (a Modrinth version number, a CurseForge file's
  // display name).
  const repairTarget = createMemo(() => {
    const version = pendingSetup.data?.repair_version ?? null
    const pack = instanceDetails.data?.modpack?.modpack
    if (!pending() || version === null || !pack) return null
    if (pack.type === "modrinth") {
      return version === pack.value.version_id
        ? null
        : { type: "modrinth" as const, version }
    }
    const fileId = Number(version)
    if (version === String(pack.value.file_id)) return null
    return {
      type: "curseforge" as const,
      modId: pack.value.project_id,
      fileId:
        /^[1-9]\d*$/.test(version) && Number.isSafeInteger(fileId) ? fileId : 0
    }
  })
  const modrinthTarget = rspc.createQuery(() => {
    const target = repairTarget()
    return {
      queryKey: [
        "modplatforms.modrinth.getVersion",
        target?.type === "modrinth" ? target.version : ""
      ],
      enabled: target?.type === "modrinth"
    }
  })
  const curseforgeTarget = rspc.createQuery(() => {
    const target = repairTarget()
    const file =
      target?.type === "curseforge"
        ? { modId: target.modId, fileId: target.fileId }
        : { modId: 0, fileId: 0 }
    return {
      queryKey: ["modplatforms.curseforge.getModFile", file],
      enabled: target?.type === "curseforge" && target.fileId > 0
    }
  })
  // What the pending block names the version Repair installs by, never a
  // raw platform id: without a name, a sentence that needs none is shown.
  const repairVersionName = (): string | undefined => {
    const target = repairTarget()
    if (target === null) return modpackInfo.data?.version_name || undefined
    if (target.type === "modrinth") {
      const version = modrinthTarget.data
      return version?.id === target.version
        ? version.version_number || undefined
        : undefined
    }
    const file = curseforgeTarget.data?.data
    return file?.id === target.fileId
      ? file.displayName || undefined
      : undefined
  }
  // The text is rendered as text, so a version name like "Farmer's Pack
  // 1.2/Forge" is passed through unescaped instead of as &#39; and &#x2F;.
  const pendingText = () => {
    const version = repairVersionName()
    const other = repairTarget() !== null
    const options = { version, interpolation: { escapeValue: false } }
    if (!previewWithheld()) {
      return version
        ? t("instances:_trn_repair_pending_discard", options)
        : t("instances:_trn_repair_pending_discard_unnamed")
    }
    if (other) {
      return version
        ? t("instances:_trn_repair_pending_withheld_target", options)
        : t("instances:_trn_repair_pending_withheld_target_unnamed")
    }
    return version
      ? t("instances:_trn_repair_pending_withheld", options)
      : t("instances:_trn_repair_pending_withheld_unnamed")
  }

  const checkOriginMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.checkPackOrigin"],
    onSuccess: (taskId) => setOriginTaskId(taskId),
    onError: (error) => {
      toast.error(t("general:_trn_error"), { description: error.message })
    }
  }))
  const originTask = rspc.createQuery(() => ({
    queryKey: ["vtask.getTask", originTaskId()],
    enabled: originTaskId() !== null
  }))
  createEffect(() => {
    if (originTaskId() !== null && originTask.data === null) {
      setOriginTaskId(null) // task done; preview invalidation refetches verdicts
    }
  })
  const originPercentage = createMemo(() => {
    const progress = originTask.data?.progress
    return progress?.type === "Known"
      ? Math.round(progress.value * 100)
      : undefined
  })

  const countRows = createMemo(() => {
    const c = selectedVariant()?.counts
    return (
      [
        ["instances:_trn_repair_counts_modified", c?.restore_modified ?? 0],
        ["instances:_trn_repair_counts_deleted", c?.restore_deleted ?? 0],
        ["instances:_trn_repair_counts_unchanged", c?.unchanged ?? 0],
        ["instances:_trn_repair_counts_disabled_kept", c?.disabled_kept ?? 0],
        ["instances:_trn_repair_counts_re_enabled", c?.re_enabled ?? 0]
      ] as const
    ).filter(([, count]) => count > 0)
  })

  // Paths a completed origin check proved were shipped by *some* published
  // version of the pack — "proven stale" rather than merely unrecognised.
  // Excludes anything the backend wouldn't actually delete (`!deletable`),
  // same rule the per-row checkbox uses.
  const provenPaths = createMemo(() =>
    (preview.data?.untracked ?? [])
      .filter(
        (file) =>
          file.deletable &&
          file.selection?.proven &&
          file.origin !== null &&
          typeof file.origin === "object" &&
          "ShippedIn" in file.origin
      )
      .map((file) => file.selection!)
  )

  createEffect(() => {
    const rows = preview.data?.untracked ?? []
    setTicked((previous) => reconcileSelections(previous, rows))
  })

  const repairInstanceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.repairModpack"],
    onSuccess: () => {
      toast.success(t("instances:_trn_repair_started"))
    },
    meta: LOCAL_ERROR_TOAST,
    onError: (error) => {
      toastMutationError(t("instances:_trn_repair_failed"), error)
    }
  }))

  const repairServerMutation = rspc.createMutation(() => ({
    mutationKey: ["server.reinstallServer"],
    onSuccess: () => {
      toast.success(t("instances:_trn_repair_started"))
    },
    meta: LOCAL_ERROR_TOAST,
    onError: (error) => {
      toastMutationError(t("instances:_trn_repair_failed"), error)
    }
  }))

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      width="w-110"
      // The server branch's content is short and static, so it keeps
      // ModalLayout's own default sizing (`h-full` +
      // `overflow-hidden`). The instance branch grows with the preview
      // (counts, expandable file list, untracked files, duplicates), which
      // can easily exceed that default's clipping `overflow-hidden` — cap
      // the height and make it scroll instead of silently hiding content.
      height={isServer() ? undefined : "h-[640px] max-h-[85vh]"}
      scrollable={isServer() ? undefined : "overflow-y-auto overflow-x-hidden"}
    >
      <div class="flex flex-col gap-5">
        <div class="text-lightSlate-50">
          <Trans
            key="instances:_trn_repair_question"
            options={{ name: props.data?.name }}
          >
            {""}
            <span class="font-bold" />
            {""}
          </Trans>
        </div>

        <p class="text-lightSlate-300 m-0 text-sm leading-relaxed">
          {t(
            isServer()
              ? "instances:_trn_repair_intro_server"
              : "instances:_trn_repair_intro_instance"
          )}
        </p>

        {/* Without a preview (withheld, or hidden while an install or
            update runs) there is no re-enable option or file list below to
            point at. */}
        <p class="text-lightSlate-300 m-0 text-sm leading-relaxed">
          {t(
            isServer()
              ? "instances:_trn_repair_replaced_server"
              : previewWithheld() || installing()
                ? "instances:_trn_repair_replaced_instance_withheld"
                : "instances:_trn_repair_replaced_instance"
          )}
        </p>

        <p class="text-lightSlate-300 m-0 text-sm leading-relaxed">
          {t(
            isServer()
              ? "instances:_trn_repair_kept_server"
              : installing()
                ? "instances:_trn_repair_kept_instance_busy"
                : previewWithheld()
                  ? "instances:_trn_repair_kept_instance_withheld"
                  : "instances:_trn_repair_kept_instance"
          )}
        </p>

        <Show when={pending()}>
          <div class="flex flex-col gap-3" data-testid="repair-pending-setup">
            <p class="text-lightSlate-50 m-0 font-bold">
              {t(
                busy()
                  ? "instances:_trn_repair_pending_busy_title"
                  : "instances:_trn_repair_pending_title"
              )}
            </p>
            <p class="text-lightSlate-300 m-0 text-sm leading-relaxed">
              {busy() ? t("instances:_trn_repair_pending_busy") : pendingText()}
            </p>
            <Show
              when={
                !busy() &&
                (pendingSetup.data?.can_resume || pendingSetup.data?.can_cancel)
              }
            >
              <p class="text-lightSlate-300 m-0 text-sm leading-relaxed">
                {t(
                  pendingSetup.data?.can_cancel
                    ? "instances:_trn_repair_pending_alternatives"
                    : "instances:_trn_repair_pending_resume_hint"
                )}
              </p>
              <div class="flex flex-wrap gap-2">
                <Show when={pendingSetup.data?.can_resume}>
                  <Button
                    type="secondary"
                    data-testid="repair-resume-setup"
                    disabled={resumeSetup.isPending || cancelPending.isPending}
                    loading={resumeSetup.isPending}
                    onClick={() => {
                      if (!pendingSetup.data?.can_resume) return
                      resumeSetup.mutate(instanceId())
                    }}
                  >
                    {t("instances:_trn_repair_pending_resume")}
                  </Button>
                </Show>
                <Show when={pendingSetup.data?.can_cancel}>
                  <Button
                    type="secondary"
                    data-testid="repair-cancel-pending"
                    disabled={cancelPending.isPending || resumeSetup.isPending}
                    loading={cancelPending.isPending}
                    onClick={() => {
                      const operation = pendingSetup.data?.operation
                      if (operation && pendingSetup.data?.can_cancel) {
                        cancelPending.mutate({
                          instance: instanceId(),
                          expected_operation: operation
                        })
                      }
                    }}
                  >
                    {t("instances:_trn_repair_pending_cancel")}
                  </Button>
                </Show>
              </div>
            </Show>
          </div>
        </Show>
        <Show when={!isServer() && pendingSetup.isError}>
          <p class="m-0 text-red-400">
            {t("instances:_trn_repair_pending_status_failed")}
          </p>
        </Show>
        <Show
          when={
            !isServer() &&
            pendingSetup.isSuccess &&
            !previewWithheld() &&
            !installing()
          }
        >
          <div class="border-darkSlate-600 flex flex-col gap-4 border-0 border-t border-solid pt-4">
            <Show
              when={!preview.isLoading}
              fallback={
                <div class="flex items-center justify-center py-6">
                  <Spinner />
                </div>
              }
            >
              <div
                class="flex flex-col gap-1 transition-opacity"
                classList={{ "opacity-60": preview.isFetching }}
                data-testid="repair-preview-counts"
              >
                <For each={countRows()}>
                  {([key, count]) => (
                    <div class="text-lightSlate-300 text-sm">
                      {t(key, { count })}
                    </div>
                  )}
                </For>
              </div>

              <Show when={preview.data?.has_packinfo === false}>
                <p class="text-lightSlate-300 m-0 text-sm leading-relaxed">
                  {t("instances:_trn_repair_no_record")}
                </p>
              </Show>

              <Checkbox
                checked={reEnable()}
                onChange={(checked) => setReEnable(checked)}
              >
                <span
                  class="text-lightSlate-300 text-sm"
                  data-testid="repair-reenable-checkbox"
                >
                  {t("instances:_trn_repair_reenable_label")}
                </span>
              </Checkbox>

              <Show when={(selectedVariant()?.entries.length ?? 0) > 0}>
                <div class="flex flex-col gap-2">
                  <Button
                    type="text"
                    data-testid="repair-preview-expand"
                    onClick={() => setExpanded((e) => !e)}
                  >
                    {expanded()
                      ? t("instances:_trn_repair_hide_details")
                      : t("instances:_trn_repair_show_details")}
                  </Button>
                  <Show when={expanded()}>
                    <div class="bg-darkSlate-800 flex max-h-60 flex-col gap-1 overflow-y-auto rounded-md p-2">
                      <For each={selectedVariant()?.entries}>
                        {(entry) => (
                          <div
                            class="text-lightSlate-300 flex items-center justify-between gap-2 text-xs"
                            data-testid="repair-preview-entry"
                          >
                            <span class="truncate" title={entry.path}>
                              {entry.path}
                            </span>
                            <span class="text-lightSlate-500 shrink-0">
                              {repairReasonLabel(t, entry.reason)}
                            </span>
                          </div>
                        )}
                      </For>
                    </div>
                  </Show>
                </div>
              </Show>

              <Show when={(preview.data?.duplicates.length ?? 0) > 0}>
                <div class="flex flex-col gap-2">
                  <p class="text-lightSlate-50 m-0 text-sm font-bold">
                    {t("instances:_trn_repair_duplicates_title")}
                  </p>
                  <For each={preview.data?.duplicates}>
                    {(group) => (
                      <div class="bg-darkSlate-800 flex flex-col gap-1 rounded-md p-2">
                        <span class="text-lightSlate-100 text-xs font-bold">
                          {group.modid}
                        </span>
                        <For each={group.files}>
                          {(side) => (
                            <div class="text-lightSlate-300 flex items-center justify-between gap-2 text-xs">
                              <span class="truncate" title={side.path}>
                                {side.path}
                              </span>
                              <span class="text-lightSlate-500 shrink-0">
                                {side.pack_owned
                                  ? t(
                                      "instances:_trn_repair_duplicate_pack_owned"
                                    )
                                  : t(
                                      "instances:_trn_repair_duplicate_untracked"
                                    )}
                              </span>
                            </div>
                          )}
                        </For>
                      </div>
                    )}
                  </For>
                </div>
              </Show>

              <Show when={(preview.data?.untracked.length ?? 0) > 0}>
                <div class="flex flex-col gap-2">
                  <p class="text-lightSlate-50 m-0 text-sm font-bold">
                    {t("instances:_trn_repair_untracked_title")}
                  </p>
                  <p class="text-lightSlate-400 m-0 text-xs">
                    {t("instances:_trn_repair_untracked_hint")}
                  </p>

                  <div class="flex flex-wrap items-center gap-2">
                    <Show when={isModrinthPack()}>
                      <Button
                        type="secondary"
                        size="small"
                        data-testid="repair-check-origin"
                        loading={
                          checkOriginMutation.isPending ||
                          originTaskId() !== null
                        }
                        percentage={originPercentage()}
                        onClick={() => checkOriginMutation.mutate(instanceId())}
                      >
                        <div class="i-hugeicons:search-01" />
                        {t("instances:_trn_repair_check_origin")}
                      </Button>
                    </Show>
                    <Show when={provenPaths().length > 0}>
                      <Button
                        type="secondary"
                        size="small"
                        data-testid="repair-select-proven"
                        onClick={() =>
                          setTicked(
                            new Map(
                              provenPaths().map((selection) => [
                                selection.path,
                                freezeSelection(selection)
                              ])
                            )
                          )
                        }
                      >
                        <div class="i-hugeicons:checkmark-circle-02" />
                        {t("instances:_trn_repair_select_proven")}
                      </Button>
                    </Show>
                  </div>

                  <div class="flex max-h-48 flex-col gap-1 overflow-y-auto">
                    <For each={preview.data?.untracked}>
                      {(file) => (
                        <div
                          class="flex items-start gap-3 py-1.5"
                          data-testid="repair-untracked-row"
                        >
                          <div class="flex h-5 w-5 shrink-0 items-center justify-center">
                            <Show
                              when={
                                file.deletable &&
                                file.selection !== null &&
                                file.origin !== "CurrentVersion"
                              }
                            >
                              <div data-testid="repair-untracked-checkbox">
                                <Checkbox
                                  checked={ticked().has(file.path)}
                                  onChange={() => toggleTicked(file)}
                                />
                              </div>
                            </Show>
                          </div>
                          <div class="flex min-w-0 flex-1 flex-col gap-0.5">
                            <span
                              class="text-lightSlate-50 truncate text-sm"
                              title={file.path}
                            >
                              {file.path}
                            </span>
                            <span class="text-lightSlate-400 flex flex-wrap items-center gap-x-2 text-xs">
                              <span>{formatBytes(file.size)}</span>
                              <Show when={file.label === "DisabledPackFile"}>
                                <span>
                                  {t(
                                    "instances:_trn_repair_untracked_disabled_pack_file"
                                  )}
                                </span>
                              </Show>
                              <Show when={originVerdictText(t, file.origin)}>
                                {(text) => <span>{text()}</span>}
                              </Show>
                            </span>
                          </div>
                        </div>
                      )}
                    </For>
                  </div>
                </div>
              </Show>
            </Show>
          </div>
        </Show>

        <Show when={busy() && !pending()}>
          <p
            class="text-lightSlate-300 m-0 text-sm leading-relaxed"
            data-testid="repair-busy"
          >
            {t("instances:_trn_repair_busy")}
          </p>
        </Show>

        <div class="flex w-full justify-between">
          <Button
            onClick={() => {
              modalsContext?.closeModal()
            }}
          >
            <div class="i-hugeicons:cancel-01" />
            {t("instances:_trn_repair_cancel")}
          </Button>
          <Button
            type="secondary"
            data-testid="repair-modpack-confirm"
            disabled={
              !isServer() &&
              (!pendingSetup.isSuccess ||
                pendingSetup.data?.pending ||
                preview.isLoading ||
                preview.isError)
            }
            onClick={() => {
              modalsContext?.closeModal()
              navigateAwayIfInsideDetail()
              if (isServer()) {
                repairServerMutation.mutate(props?.data?.id)
              } else if (status) {
                // Without a preview nothing was shown to select or re-enable.
                const withheld = status.preview_withheld
                repairInstanceMutation.mutate({
                  instance: instanceId(),
                  cleanup_paths: [],
                  cleanup_selections: withheld ? [] : [...ticked().values()],
                  re_enable_disabled: withheld ? false : reEnable(),
                  // Sent back as the core gave it.
                  expected_pending: status.discard_token ?? null
                })
              }
            }}
          >
            <div class="i-hugeicons:refresh" />
            {t(
              pending() && !busy()
                ? "instances:_trn_repair_confirm_discard"
                : "instances:_trn_repair_confirm"
            )}
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default RepairModpack
