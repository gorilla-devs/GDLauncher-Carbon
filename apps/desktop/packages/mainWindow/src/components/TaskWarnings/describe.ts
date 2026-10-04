import type {
  FEDependencyPlatform,
  FEPreviousPackFilesReason,
  FESkippedDependencyReason,
  FETaskWarning,
  FETaskWarnings
} from "@gd/core_module/bindings"
import type { NamespacedTranslationKey } from "@gd/i18n"

export type Translate = (
  key: NamespacedTranslationKey,
  options?: Record<string, unknown>
) => string

export interface WarningToast {
  title: string
  lines: string[]
  hint: string
}

const SKIP_REASON: Record<FESkippedDependencyReason, NamespacedTranslationKey> =
  {
    NoCompatibleVersion:
      "notifications:_trn_addon_dependency_skipped_no_compatible_version",
    NotDownloadable:
      "notifications:_trn_addon_dependency_skipped_not_downloadable",
    Unavailable: "notifications:_trn_addon_dependency_skipped_unavailable",
    NotForServers:
      "notifications:_trn_addon_dependency_skipped_not_for_servers",
    RemovedDuringInstall: "notifications:_trn_addon_dependency_skipped_removed",
    Disabled: "notifications:_trn_addon_dependency_skipped_disabled"
  }

const UNNAMED: Record<FEDependencyPlatform, NamespacedTranslationKey> = {
  Curseforge: "notifications:_trn_addon_dependency_unnamed_curseforge",
  Modrinth: "notifications:_trn_addon_dependency_unnamed_modrinth"
}

const PREVIOUS_PACK_FILES: Record<
  FEPreviousPackFilesReason,
  NamespacedTranslationKey
> = {
  VersionUnavailable: "notifications:_trn_modpack_previous_version_unavailable",
  VersionUnreadable: "notifications:_trn_modpack_previous_version_unreadable",
  SomeFilesUnchecked: "notifications:_trn_modpack_previous_files_unchecked"
}

const PLATFORM_NAME: Record<FEDependencyPlatform, string> = {
  Curseforge: "CurseForge",
  Modrinth: "Modrinth"
}

// Names come from CurseForge, Modrinth and the user's own instance names, and
// sonner renders the toast as text, so HTML escaping would show "&#39;".
const noEscape = { escapeValue: false }

function taskArgs(entry: FETaskWarnings): Record<string, unknown> {
  return "args" in entry.name ? entry.name.args : {}
}

/** The addon a task was about: its project title when the core names it
 * (Modrinth version names are often a bare "4.0"), else the task name's. */
export function warnedSubject(entry: FETaskWarnings, t: Translate): string {
  if (entry.subject) return entry.subject
  const args = taskArgs(entry)
  const name = args.mod_name ?? args.world_name
  return typeof name === "string" && name
    ? name
    : t("notifications:_trn_addon_fallback_name")
}

interface NamedDependency {
  name: string | null
  project_id: string
  platform: FEDependencyPlatform
}

/** How a dependency is named: its title and id, the id alone when that is
 * all there is (a file name), or the platform's project id when the platform
 * no longer knows the project. */
export function dependencyLabel(
  warning: NamedDependency,
  t: Translate
): string {
  const { name, project_id: project, platform } = warning
  if (!name) return t(UNNAMED[platform], { project, interpolation: noEscape })
  if (name === project) return name
  return t("notifications:_trn_addon_dependency_label", {
    name,
    project,
    interpolation: noEscape
  })
}

function describeWarning(warning: FETaskWarning, t: Translate): string {
  switch (warning.type) {
    case "SkippedDependency":
      return t(SKIP_REASON[warning.value.reason], {
        dependency: dependencyLabel(warning.value, t),
        interpolation: noEscape
      })
    case "ReplacedPinnedVersion":
      // Only Modrinth dependencies pin a version.
      return t("notifications:_trn_addon_dependency_replaced_version", {
        dependency: dependencyLabel(
          { ...warning.value, platform: "Modrinth" },
          t
        ),
        pinned: warning.value.pinned,
        installed: warning.value.installed,
        interpolation: noEscape
      })
    case "ConflictingVersions":
      return t("notifications:_trn_addon_dependency_conflicting_versions", {
        dependency: dependencyLabel(warning.value, t),
        installed: warning.value.installed,
        interpolation: noEscape
      })
    case "PackFilesSkippedAtLink":
      return t("notifications:_trn_pack_files_skipped_at_link", {
        path: warning.value.path,
        interpolation: noEscape
      })
    case "WorldNameTaken":
      return t("notifications:_trn_world_name_taken", {
        name: warning.value.name,
        saved_as: warning.value.saved_as,
        interpolation: noEscape
      })
    case "PreviousPackFilesKept":
      return t(PREVIOUS_PACK_FILES[warning.value.reason], {
        platform: PLATFORM_NAME[warning.value.platform]
      })
  }
}

function nonEmpty(value: unknown): string | null {
  return typeof value === "string" && value !== "" ? value : null
}

/** Toast content for a succeeded task's warnings: which required
 * dependencies were left out, are installed but disabled, or were installed
 * at another version than asked,
 * which of a modpack's files were not installed, which files a modpack
 * version change kept, or under which name a world was saved because its
 * own was taken, why, and what the user can do about it. */
export function describeTaskWarnings(
  entry: FETaskWarnings,
  t: Translate
): WarningToast | null {
  const lines = entry.warnings.map((warning) => describeWarning(warning, t))
  if (lines.length === 0) return null
  const args = taskArgs(entry)
  // A modpack version change runs as the instance's preparation task, whose
  // name carries the instance's name.
  if (
    entry.warnings.every((warning) => warning.type === "PreviousPackFilesKept")
  ) {
    const instance = nonEmpty(args.name)
    return {
      title:
        instance !== null
          ? t("notifications:_trn_modpack_update_kept_files_in_instance", {
              instance,
              interpolation: noEscape
            })
          : t("notifications:_trn_modpack_update_kept_files"),
      lines,
      hint: t("notifications:_trn_modpack_update_kept_files_hint")
    }
  }
  const title = warnedSubject(entry, t)
  const instance = nonEmpty(args.instance_name)
  const server = nonEmpty(args.server_name)
  // A missing dependency is the more likely reason the game will not start.
  const skipped = entry.warnings.some(
    (warning) => warning.type === "SkippedDependency"
  )
  if (server !== null) {
    // Only a server's modpack install raises this, and never with the others.
    if (
      entry.warnings.every(
        (warning) => warning.type === "PackFilesSkippedAtLink"
      )
    ) {
      return {
        title: t("notifications:_trn_pack_files_skipped_on_server", {
          server,
          interpolation: noEscape
        }),
        lines,
        hint: t("notifications:_trn_pack_files_skipped_hint")
      }
    }
    return {
      title: t(
        skipped
          ? "notifications:_trn_addon_dependencies_skipped_on_server"
          : "notifications:_trn_addon_dependencies_replaced_on_server",
        { title, server, interpolation: noEscape }
      ),
      lines,
      hint: t(
        skipped
          ? "notifications:_trn_addon_dependencies_skipped_server_hint"
          : "notifications:_trn_addon_dependencies_replaced_server_hint",
        { title, interpolation: noEscape }
      )
    }
  }
  // A world installed beside a save that already had its name; a world
  // install raises nothing else.
  if (entry.warnings.every((warning) => warning.type === "WorldNameTaken")) {
    return {
      title:
        instance !== null
          ? t("notifications:_trn_world_saved_as_copy_in_instance", {
              title,
              instance,
              interpolation: noEscape
            })
          : t("notifications:_trn_world_saved_as_copy", {
              title,
              interpolation: noEscape
            }),
      lines,
      hint: t("notifications:_trn_world_saved_as_copy_hint")
    }
  }
  // Required dependencies the user has disabled: they are installed, and
  // enabling them is the way out.
  const disabled = entry.warnings.every(
    (warning) =>
      warning.type === "SkippedDependency" &&
      warning.value.reason === "Disabled"
  )
  const key: NamespacedTranslationKey = disabled
    ? instance !== null
      ? "notifications:_trn_addon_dependencies_disabled_in_instance"
      : "notifications:_trn_addon_dependencies_disabled"
    : skipped
      ? instance !== null
        ? "notifications:_trn_addon_dependencies_skipped_in_instance"
        : "notifications:_trn_addon_dependencies_skipped"
      : instance !== null
        ? "notifications:_trn_addon_dependencies_replaced_in_instance"
        : "notifications:_trn_addon_dependencies_replaced"
  return {
    title: t(
      key,
      instance !== null
        ? { title, instance, interpolation: noEscape }
        : { title, interpolation: noEscape }
    ),
    lines,
    hint: t(
      disabled
        ? "notifications:_trn_addon_dependencies_disabled_hint"
        : skipped
          ? "notifications:_trn_addon_dependencies_skipped_hint"
          : "notifications:_trn_addon_dependencies_replaced_hint",
      { title, interpolation: noEscape }
    )
  }
}
