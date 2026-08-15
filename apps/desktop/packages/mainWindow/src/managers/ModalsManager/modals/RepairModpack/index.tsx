import { createEffect, createMemo, createSignal, For, Show } from "solid-js"
import { keepPreviousData } from "@tanstack/solid-query"
import { rspc } from "@/utils/rspcClient"
import { ModalProps, useModal } from "../.."
import ModalLayout from "../../ModalLayout"
import { Button, Checkbox, Spinner, toast } from "@gd/ui"
import { Trans, useTransContext, type TypedTFunction } from "@gd/i18n"
import { useLocation } from "@solidjs/router"
import { useGDNavigate } from "@/managers/NavigationManager"
import { formatBytes } from "@/utils/formatBytes"
import { FEOriginVerdict, FERepairReason } from "@gd/core_module/bindings"

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
  const [ticked, setTicked] = createSignal<Set<string>>(new Set(), {
    equals: false
  })
  const [expanded, setExpanded] = createSignal(false)
  const [originTaskId, setOriginTaskId] = createSignal<number | null>(null)

  const toggleTicked = (path: string) => {
    setTicked((prev) => {
      const next = new Set(prev)
      if (next.has(path)) {
        next.delete(path)
      } else {
        next.add(path)
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

  // Keyed on the instance alone: the backend returns both
  // `re_enable_disabled` outcomes (`with_re_enable`/`without_re_enable`) in
  // one response, computed over a single disk scan — see `RepairPreview`'s
  // own doc (`managers/instance/modpack/mod.rs`). Toggling the checkbox
  // below is therefore a pure client-side selection between the two
  // already-fetched variants (`selectedVariant` below), never a refetch.
  const preview = rspc.createQuery(() => ({
    queryKey: ["instance.getRepairPreview", { instance: instanceId() }],
    enabled: !isServer(),
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
          file.origin !== null &&
          typeof file.origin === "object" &&
          "ShippedIn" in file.origin
      )
      .map((file) => file.path)
  )

  const repairInstanceMutation = rspc.createMutation(() => ({
    mutationKey: ["instance.repairModpack"],
    onSuccess: () => {
      toast.success(t("instances:_trn_repair_started"))
    },
    onError: (error) => {
      toast.error(t("instances:_trn_repair_failed"), {
        description: error.message
      })
    }
  }))

  const repairServerMutation = rspc.createMutation(() => ({
    mutationKey: ["server.reinstallServer"],
    onSuccess: () => {
      toast.success(t("instances:_trn_repair_started"))
    },
    onError: (error) => {
      toast.error(t("instances:_trn_repair_failed"), {
        description: error.message
      })
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

        <p class="text-lightSlate-300 m-0 text-sm leading-relaxed">
          {t(
            isServer()
              ? "instances:_trn_repair_replaced_server"
              : "instances:_trn_repair_replaced_instance"
          )}
        </p>

        <p class="text-lightSlate-300 m-0 text-sm leading-relaxed">
          {t(
            isServer()
              ? "instances:_trn_repair_kept_server"
              : "instances:_trn_repair_kept_instance"
          )}
        </p>

        <Show when={!isServer()}>
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
                        onClick={() => setTicked(new Set(provenPaths()))}
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
                                file.origin !== "CurrentVersion"
                              }
                            >
                              <div data-testid="repair-untracked-checkbox">
                                <Checkbox
                                  checked={ticked().has(file.path)}
                                  onChange={() => toggleTicked(file.path)}
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
            disabled={!isServer() && preview.isLoading}
            onClick={() => {
              modalsContext?.closeModal()
              navigateAwayIfInsideDetail()
              if (isServer()) {
                repairServerMutation.mutate(props?.data?.id)
              } else {
                repairInstanceMutation.mutate({
                  instance: instanceId(),
                  cleanup_paths: [...ticked()],
                  re_enable_disabled: reEnable()
                })
              }
            }}
          >
            <div class="i-hugeicons:refresh" />
            {t("instances:_trn_repair_confirm")}
          </Button>
        </div>
      </div>
    </ModalLayout>
  )
}

export default RepairModpack
