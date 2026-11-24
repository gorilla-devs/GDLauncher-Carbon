import type {
  FEJavaEntry,
  FEJavaGroup,
  FEJavaOverview,
  FEJavaUsage,
  FEManagedJavaInstall,
  FERemovedUnusedJavas
} from "@gd/core_module/bindings"
import {
  minecraftRange,
  minecraftRangeLabel,
  versionKey,
  type CatalogLabelKey
} from "../../../managers/ModalsManager/modals/Java/managedJavaCatalog"

type OverviewLabelKey =
  | CatalogLabelKey
  | "java:_trn_automatic_picks_for_minecraft"
  | "java:_trn_make_automatic_pick_for_minecraft"
  | "java:_trn_automatic_picks_for_major"
  | "java:_trn_make_automatic_pick_for_major"
  | "java:_trn_a11y_automatic_picks_for"
  | "java:_trn_a11y_make_automatic_pick_for"
  | "java:_trn_a11y_automatic_picks_for_major"
  | "java:_trn_a11y_make_automatic_pick_for_major"
  | "java:_trn_installed_by_gdlauncher"
  | "java:_trn_installed_by_gdlauncher_size"
  | "java:_trn_on_this_computer"
  | "java:_trn_added_by_you"
  | "java:_trn_delete_frees_size"
  | "java:_trn_remove_from_list"
  | "java:_trn_close_game_first"
  | "java:_trn_removed_when_game_closes"
  | "java:_trn_removed_javas"
  | "java:_trn_javas_removed_when_game_closes"
  | "general:_trn_delete"

/** The slice of i18next's `t` these helpers need, so tests can pass a stub. */
export type OverviewTranslate = (
  key: OverviewLabelKey,
  options: Record<string, string | number>
) => string

type FormatBytes = (bytes: number) => string

/** "Java 21 · Minecraft 1.20.5 – 1.21.11", or "Java 21" without version
 *  data. */
export function groupHeader(g: FEJavaGroup, t: OverviewTranslate): string {
  return minecraftRangeLabel(g, t) ?? t("java:_trn_major", { major: g.major })
}

/** The tooltip and accessible name of a row's marker: the Java Automatic
 *  picks for the group, or the button that makes it pick `name`. */
export function markerText(
  g: FEJavaGroup,
  name: string,
  inUse: boolean,
  t: OverviewTranslate
): { tooltip: string; label: string } {
  const range = minecraftRange(g, t)
  if (range !== undefined) {
    return inUse
      ? {
          tooltip: t("java:_trn_automatic_picks_for_minecraft", { range }),
          label: t("java:_trn_a11y_automatic_picks_for", { name, range })
        }
      : {
          tooltip: t("java:_trn_make_automatic_pick_for_minecraft", { range }),
          label: t("java:_trn_a11y_make_automatic_pick_for", { name, range })
        }
  }

  const major = g.major
  return inUse
    ? {
        tooltip: t("java:_trn_automatic_picks_for_major", { major }),
        label: t("java:_trn_a11y_automatic_picks_for_major", { name, major })
      }
    : {
        tooltip: t("java:_trn_make_automatic_pick_for_major", { major }),
        label: t("java:_trn_a11y_make_automatic_pick_for_major", {
          name,
          major
        })
      }
}

/** A known issue no one needs warning about: a found or added Java that
 *  nothing uses, which the user updates where it was installed. GDLauncher's
 *  own Javas stay flagged, as their fix is one click away. Nothing is muted
 *  until `usageComplete` (the startup instance scan is done), since until
 *  then "nothing uses it" may only mean "not read yet". */
export function advisoryMuted(
  e: Pick<FEJavaEntry, "source" | "usedBy">,
  usageComplete: boolean
): boolean {
  return usageComplete && e.usedBy.length === 0 && e.source !== "managed"
}

/** The muted second line of a row: where the Java comes from, and its path
 *  when the user would need it to tell Javas apart. */
export function sourceLine(
  e: FEJavaEntry,
  t: OverviewTranslate,
  formatBytes: FormatBytes
): { text: string; path?: string } {
  switch (e.source) {
    case "managed":
      return {
        text:
          e.sizeBytes !== null
            ? t("java:_trn_installed_by_gdlauncher_size", {
                size: formatBytes(e.sizeBytes)
              })
            : t("java:_trn_installed_by_gdlauncher", {})
      }
    case "local":
      return { text: t("java:_trn_on_this_computer", {}), path: e.path }
    case "custom":
      return { text: t("java:_trn_added_by_you", {}), path: e.path }
    default: {
      const unhandled: never = e.source
      return unhandled
    }
  }
}

const sameVersion = (a: string, b: string) =>
  (versionKey(a) ?? a) === (versionKey(b) ?? b)

/** Installs of `major` still running, minus those whose Java is already
 *  listed (it shows up just before the install ends). */
export function installRowsFor(
  major: number,
  installs: FEManagedJavaInstall[],
  entries: FEJavaEntry[]
): FEManagedJavaInstall[] {
  return installs.filter(
    (i) =>
      i.major === major &&
      !entries.some(
        (e) => e.source === "managed" && sameVersion(e.version, i.version)
      )
  )
}

/** What the delete icon does, or `null` for Javas found on the computer,
 *  which have none. */
export function deleteTooltip(
  e: FEJavaEntry,
  t: OverviewTranslate,
  formatBytes: FormatBytes
): string | null {
  if (e.source === "local") return null
  if (e.pendingRemoval) return t("java:_trn_removed_when_game_closes", {})
  if (e.runningGame) return t("java:_trn_close_game_first", {})

  if (e.source === "custom") return t("java:_trn_remove_from_list", {})
  return e.sizeBytes !== null
    ? t("java:_trn_delete_frees_size", { size: formatBytes(e.sizeBytes) })
    : t("general:_trn_delete", {})
}

/** Deleting asks first when instances use the Java, since they move to
 *  Automatic. */
export function deleteNeedsConfirm(e: FEJavaEntry): boolean {
  return e.usedBy.length > 0
}

/** A Java's users as the delete confirm lists them: the instances that use
 *  it directly or through Automatic, then each custom profile with how many
 *  instances use it through that profile, in order of first appearance. */
export function profileUsers(usedBy: FEJavaUsage[]): {
  instances: FEJavaUsage[]
  profiles: { name: string; count: number }[]
} {
  const counts = new Map<string, number>()
  for (const usage of usedBy) {
    if (usage.viaProfile === null) continue
    counts.set(usage.viaProfile, (counts.get(usage.viaProfile) ?? 0) + 1)
  }

  return {
    instances: usedBy.filter((usage) => usage.viaProfile === null),
    profiles: [...counts].map(([name, count]) => ({ name, count }))
  }
}

/** Every Java in the list once, in the list's order. */
export function javaChoices(o: FEJavaOverview): FEJavaEntry[] {
  const seen = new Set<string>()
  return [...o.groups.flatMap((g) => g.javas), ...o.otherJavas].filter((e) => {
    if (seen.has(e.id)) return false
    seen.add(e.id)
    return true
  })
}

/** Ids of the Javas `advisoryId` affects, in the list's order. */
export function javasWithAdvisory(
  o: FEJavaOverview,
  advisoryId: string
): string[] {
  return javaChoices(o)
    .filter((e) => e.knownIssues.some((i) => i.advisoryId === advisoryId))
    .map((e) => e.id)
}

/** Where the advisory toast's "Review" lands: the Javas the toast counted
 *  that still have the advisory, in the list's order, or every Java with it
 *  when the list has none of those. Muted ones (see `advisoryMuted`) never
 *  count. `undefined` while `stale` data has none of them, since the fresh
 *  list may. */
export function reviewTargets(
  o: FEJavaOverview,
  advisoryId: string,
  counted: string[],
  stale: boolean
): string[] | undefined {
  const muted = new Set(
    javaChoices(o)
      .filter((e) => advisoryMuted(e, o.usageComplete))
      .map((e) => e.id)
  )
  const flagged = javasWithAdvisory(o, advisoryId).filter(
    (id) => !muted.has(id)
  )
  const wanted = new Set(counted)
  const targets = flagged.filter((id) => wanted.has(id))

  if (targets.length > 0) return targets
  return stale ? undefined : flagged
}

/** The Javas "Remove unused" removes, in the list's order. */
export function unusedJavas(o: FEJavaOverview): FEJavaEntry[] {
  const unused = new Set(o.unusedJavaIds)
  return javaChoices(o).filter((e) => unused.has(e.id))
}

/** How many instances name the custom profile `name`, or `null` without the
 *  overview (loading, or it failed), when deleting must ask rather than guess
 *  none. */
export function customProfileUsers(
  o: FEJavaOverview | undefined,
  name: string
): number | null {
  return o ? (o.customProfileUsers[name] ?? 0) : null
}

/** What "Add from a file" lets the user pick. Windows and Linux can't pick
 *  files and folders in one dialog (they show a folder picker), so only macOS
 *  gets folders, for `.jdk` bundles and Java homes. Elsewhere the binary
 *  stands for its Java home. */
export function addFromFileDialogProperties(
  platform: string
): ("openFile" | "openDirectory")[] {
  return platform.startsWith("Mac")
    ? ["openFile", "openDirectory"]
    : ["openFile"]
}

/** The toast after "Remove unused": while a game runs, the Javas go once it
 *  closes. */
export function removedUnusedMessage(
  removed: FERemovedUnusedJavas,
  t: OverviewTranslate
): string {
  return removed.deferred
    ? t("java:_trn_javas_removed_when_game_closes", { count: removed.count })
    : t("java:_trn_removed_javas", { count: removed.count })
}
