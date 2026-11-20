import type {
  FEJavaEntry,
  FEJavaOverride,
  FEJavaProfile,
  InstanceDetails
} from "@gd/core_module/bindings"
import { compareJavaVersions } from "../../../../../managers/ModalsManager/modals/Java/managedJavaCatalog"
import { javaHomeName, javaNeedText } from "../../../../../utils/java/javaText"

export { javaHomeName }

/** One item of the instance's Java dropdown. `fits` is false exactly when
 *  choosing it is what the core calls a mismatch, so it gets marked. A
 *  download names the build it must stay below, if any (Forge 1.16.5). */
export type JavaOption =
  | { kind: "automatic" }
  | { kind: "java"; entry: FEJavaEntry; fits: boolean }
  | { kind: "download"; major: number; below: string | null }
  | { kind: "profile"; name: string; major?: number; fits?: boolean }
  | { kind: "file" }

/** What Automatic would download for the instance, when no Java that fits
 *  is installed. */
export interface JavaDownload {
  major: number
  below: string | null
}

export interface JavaOptionSection {
  header?: string
  options: JavaOption[]
}

/** What the instance's Minecraft version needs: the major Automatic gives
 *  it, the build it must stay below if any, and every major it runs on
 *  (16 and 17 for Minecraft 1.17). With a Java `floor`, which a wrong-Java
 *  crash raised the need to, every major from it up. */
export interface JavaNeed {
  major: number
  below: string | null
  accepted: number[]
  floor: number | null
}

type JavaSelectLabelKey =
  | "java:_trn_major_minecraft"
  | "java:_trn_majors_minecraft"
  | "java:_trn_major_below_forge"
  | "java:_trn_other_java_versions"
  | "java:_trn_profiles"
  | "java:_trn_minecraft_needs_java"
  | "java:_trn_forge_needs_java_below"
  | "java:_trn_instance_needs_java_or_later"
  | "java:_trn_automatic"
  | "java:_trn_major_vendor_version"
  | "java:_trn_download_java_major"
  | "java:_trn_choose_a_file"

/** The slice of i18next's `t` these helpers need, so tests can pass a stub. */
export type JavaSelectTranslate = (
  key: JavaSelectLabelKey,
  options?: Record<string, string | number>
) => string

/** The need from instance details; `null` when it's unknown. */
export function javaNeed(
  details: Pick<
    InstanceDetails,
    | "requiredJavaMajor"
    | "requiredJavaBelow"
    | "acceptedJavaMajors"
    | "javaFloor"
  >
): JavaNeed | null {
  const major = details.requiredJavaMajor
  if (major === null) return null

  return {
    major,
    below: details.requiredJavaBelow,
    accepted:
      details.acceptedJavaMajors.length > 0
        ? details.acceptedJavaMajors
        : [major],
    floor: details.javaFloor
  }
}

/** Whether the instance runs on some Java of `major`, the cap aside. */
function runsOn(need: JavaNeed, major: number): boolean {
  return need.floor !== null
    ? major >= need.floor
    : need.accepted.includes(major)
}

/** The key the select compares options by. A path no listed Java has is
 *  keyed by its path, so it never matches a listed one. */
export function optionKey(option: JavaOption): string {
  switch (option.kind) {
    case "java":
      return option.entry.id
        ? `java:${option.entry.id}`
        : `path:${option.entry.path}`
    case "download":
      return `download:${option.major}`
    case "profile":
      return `profile:${option.name}`
    case "automatic":
    case "file":
      return option.kind
    default: {
      const unhandled: never = option
      return unhandled
    }
  }
}

/** The name an option shows, which typeahead matches too. A Java of the
 *  needed major goes without its major, as its group's header names it. */
export function optionLabel(
  option: JavaOption,
  need: JavaNeed | null,
  t: JavaSelectTranslate
): string {
  switch (option.kind) {
    case "automatic":
      return t("java:_trn_automatic")
    case "java": {
      const e = option.entry
      return e.major === need?.major
        ? `${e.vendorName} ${e.version}`.trim()
        : t("java:_trn_major_vendor_version", {
            major: e.major,
            vendor: e.vendorName,
            version: e.version
          })
    }
    case "download":
      return t("java:_trn_download_java_major", { major: option.major })
    case "profile":
      return option.name
    case "file":
      return t("java:_trn_choose_a_file")
    default: {
      const unhandled: never = option
      return unhandled
    }
  }
}

/** Of the needed major, at or above the build it must stay below. */
function isCapped(e: FEJavaEntry, need: JavaNeed): boolean {
  return (
    need.below !== null &&
    e.major === need.major &&
    compareJavaVersions(e.version, need.below) >= 0
  )
}

/** Whether `e` runs the instance, by the core's rule: a major it accepts,
 *  below the cap if there is one. Anything goes when the need is unknown. */
function fitsNeed(e: FEJavaEntry, need: JavaNeed | null): boolean {
  return need === null || (runsOn(need, e.major) && !isCapped(e, need))
}

/** Why `e` can't run the instance, or `undefined` when it can: what the
 *  instance needs, which for Forge 1.16.5 is always the capped Java 8. */
export function misfitNote(
  e: FEJavaEntry,
  need: JavaNeed | null,
  mcVersion: string,
  t: JavaSelectTranslate
): string | undefined {
  if (need === null || fitsNeed(e, need)) return undefined
  return javaNeedText(mcVersion, need.major, need.below, t, need.floor)
}

/** The header over the Javas the instance runs on: "Java 21 · Minecraft
 *  1.21.4", "Java 16 or 17 · Minecraft 1.17.1" where it runs on both, or
 *  "Java 8 older than 8.0.312 · Forge 1.16.5". */
function neededHeader(
  need: JavaNeed,
  mcVersion: string,
  t: JavaSelectTranslate
): string {
  if (need.floor === null && need.below !== null) {
    return t("java:_trn_major_below_forge", {
      major: need.major,
      below: need.below,
      version: mcVersion
    })
  }

  const [major, other] = [...need.accepted].sort((a, b) => a - b)
  if (need.floor === null && need.accepted.length === 2) {
    return t("java:_trn_majors_minecraft", { major, other, range: mcVersion })
  }
  return t("java:_trn_major_minecraft", {
    major: need.major,
    range: mcVersion
  })
}

/** Newest major first, then newest version, then by id so the order holds. */
const newestFirst = (a: FEJavaEntry, b: FEJavaEntry) =>
  b.major - a.major ||
  compareJavaVersions(b.version, a.version) ||
  a.id.localeCompare(b.id)

/** The options of the instance's Java dropdown, in sections:
 *  1. Automatic;
 *  2. the Javas of every major the instance runs on (16 and 17 for 1.17),
 *     under the needed major's header, newest first, with builds at or above
 *     its cap after the ones that fit; then "Download Java N" for the
 *     `download` Automatic would make, which is only set while no Java that
 *     fits is installed;
 *  3. "Other Java versions", newest major first: majors it doesn't run on;
 *  4. "Profiles", when there are custom profiles;
 *  5. "Choose a file".
 *  What doesn't fit is marked, exactly as the core would flag it. With
 *  `need` unknown, every Java is listed in one section with no header. */
export function buildJavaOptions(
  need: JavaNeed | null,
  mcVersion: string,
  entries: FEJavaEntry[],
  customProfiles: FEJavaProfile[],
  download: JavaDownload | null,
  t: JavaSelectTranslate
): { sections: JavaOptionSection[] } {
  const java = (entry: FEJavaEntry): JavaOption => ({
    kind: "java",
    entry,
    fits: fitsNeed(entry, need)
  })
  const sorted = [...entries].sort(newestFirst)
  const sections: JavaOptionSection[] = [{ options: [{ kind: "automatic" }] }]

  if (need !== null) {
    const accepted = sorted.filter((e) => runsOn(need, e.major))
    const options: JavaOption[] = [
      ...accepted.filter((e) => !isCapped(e, need)).map(java),
      ...accepted.filter((e) => isCapped(e, need)).map(java)
    ]
    if (download) options.push({ kind: "download", ...download })
    if (options.length > 0) {
      sections.push({ header: neededHeader(need, mcVersion, t), options })
    }
  }

  const others = sorted
    .filter((e) => need === null || !runsOn(need, e.major))
    .map(java)
  if (others.length > 0) {
    sections.push({
      header: need === null ? undefined : t("java:_trn_other_java_versions"),
      options: others
    })
  }

  const profiles = customProfiles.filter((p) => !p.isSystem)
  if (profiles.length > 0) {
    sections.push({
      header: t("java:_trn_profiles"),
      options: profiles.map((p) => {
        // A profile with no Java that starts runs on Automatic, as a launch
        // reads it, so it names no major and fits.
        const linked = entries.find((e) => e.id === p.javaId && e.isValid)
        return {
          kind: "profile",
          name: p.name,
          major: linked?.major,
          fits: linked === undefined || fitsNeed(linked, need)
        }
      })
    })
  }

  sections.push({ options: [{ kind: "file" }] })

  return { sections }
}

/** How `automaticIfStillMismatched` reads and writes an instance. */
export interface InstanceJavaClient {
  details: (
    instanceId: number
  ) => Promise<Pick<InstanceDetails, "javaOverrideMismatch"> | null>
  setAutomatic: (instanceId: number) => Promise<unknown>
}

/** After a Java install the user started from `instanceId`'s settings: moves
 *  that instance to Automatic, which then uses the new build, if its Java
 *  still doesn't fit. Reads the instance's details afresh, since by then the
 *  user may be anywhere. Resolves to whether it did; a failure is the
 *  caller's to report. */
export async function automaticIfStillMismatched(
  instanceId: number,
  client: InstanceJavaClient
): Promise<boolean> {
  const details = await client.details(instanceId)
  if (!details?.javaOverrideMismatch) return false

  await client.setAutomatic(instanceId)
  return true
}

/** A Java no list has, for a `Path` override to it: named after its folder,
 *  with no id, version or major. */
function unlistedJava(path: string): FEJavaEntry {
  return {
    id: "",
    path,
    version: "",
    major: 0,
    vendorName: javaHomeName(path),
    source: "custom",
    sizeBytes: null,
    isValid: true,
    nativity: "native",
    pinned: false,
    updateTo: null,
    knownIssues: [],
    usedBy: [],
    runningGame: false,
    pendingRemoval: false
  }
}

/** What the dropdown shows for the instance's `override`. Like a launch:
 *  - no override, an unset one, or the profile the instance's Minecraft
 *    version asks for (`requiredProfileId`, which older settings stored) is
 *    Automatic;
 *  - with `profiles` known, a profile that's gone, or a custom profile whose
 *    Java is gone or doesn't start, is Automatic too. */
export function currentOption(
  override: FEJavaOverride | null,
  requiredProfileId: string | null,
  entries: FEJavaEntry[],
  profiles?: FEJavaProfile[]
): StoredJavaOption {
  if (override === null) return { kind: "automatic" }

  if ("Path" in override) {
    const path = override.Path
    if (path === null) return { kind: "automatic" }
    const entry = entries.find((e) => e.path === path) ?? unlistedJava(path)
    return { kind: "java", entry, fits: true }
  }

  const name = override.Profile
  if (name === null || name === requiredProfileId) return { kind: "automatic" }

  const profile = profiles?.find((p) => p.name === name)
  const linked = profile?.javaId
    ? entries.find((e) => e.id === profile.javaId)
    : undefined
  if (profiles !== undefined) {
    const gone = profile === undefined
    const unusable = !profile?.isSystem && !linked?.isValid
    if (gone || unusable) return { kind: "automatic" }
  }

  return { kind: "profile", name, major: linked?.major }
}

/** The items that run an action rather than store a choice. */
export type JavaActionOption = Extract<
  JavaOption,
  { kind: "download" } | { kind: "file" }
>

/** The items that store a choice. */
export type StoredJavaOption = Exclude<JavaOption, JavaActionOption>

export function isAction(option: JavaOption): option is JavaActionOption {
  return option.kind === "download" || option.kind === "file"
}

/** The override that stores `option`: `null` is Automatic. */
export function overrideFor(option: StoredJavaOption): FEJavaOverride | null {
  switch (option.kind) {
    case "java":
      return { Path: option.entry.path }
    case "profile":
      return { Profile: option.name }
    case "automatic":
      return null
    default: {
      const unhandled: never = option
      return unhandled
    }
  }
}

/** Whether `name` is one of the launcher's own profiles, one per Minecraft
 *  Java version, whose ids are codenames the UI never shows. Mirrors the
 *  core's `SYSTEM_JAVA_PROFILE_NAME_PREFIX`. */
export function isSystemProfile(name: string): boolean {
  return name.startsWith("__gdl_system_java_profile__")
}
