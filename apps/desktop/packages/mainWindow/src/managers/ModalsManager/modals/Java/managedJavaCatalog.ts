import type {
  FEJavaComponent,
  FEManagedJavaBuild,
  FEManagedJavaCatalog,
  FEManagedJavaSetupProgress,
  FEMinecraftJavaMajor,
  FEVendor
} from "@gd/core_module/bindings"
import { intlLocale } from "../../../../utils/intl"

export type CatalogLabelKey =
  | "java:_trn_major_minecraft"
  | "java:_trn_minecraft_range"
  | "java:_trn_minecraft_range_newer"
  | "java:_trn_minecraft_range_older"
  | "java:_trn_major"
  | "java:_trn_major_lts"
  | "java:_trn_install_downloading"
  | "java:_trn_install_extracting"

/** The slice of i18next's `t` these helpers need, so tests can pass a stub. */
export type CatalogTranslate = (
  key: CatalogLabelKey,
  options: Record<string, string | number>
) => string

/** The majors to choose from: the catalog's, or without it (the download
 *  server is unreachable) the ones Minecraft uses, from its version data. */
const availableMajors = (
  c: FEManagedJavaCatalog | undefined,
  minecraftMajors: FEMinecraftJavaMajor[]
) => (c ? c.majors : minecraftMajors).map((m) => m.major)

/** What the major select reads: the catalog; `undefined` when the last fetch
 *  `failed`, so Minecraft's majors stand in, also while "Try again"
 *  refetches (the select doesn't empty under the user); `null` while it
 *  first loads. */
export function catalogForMajors(query: {
  data: FEManagedJavaCatalog | undefined
  failed: boolean
}): FEManagedJavaCatalog | undefined | null {
  if (query.data) return query.data
  return query.failed ? undefined : null
}

/** Picks, in order: the requested major when the catalog has it, the newest
 *  major Minecraft uses, then the newest major in the catalog. Without the
 *  catalog, from the majors Minecraft uses. */
export function defaultMajor(
  c: FEManagedJavaCatalog | undefined,
  minecraftMajors: FEMinecraftJavaMajor[],
  requested?: number
): number | undefined {
  const available = availableMajors(c, minecraftMajors)
  if (requested !== undefined && available.includes(requested)) {
    return requested
  }

  const newestFirst = (majors: number[]) => [...majors].sort((a, b) => b - a)
  const usedByMinecraft = new Set(minecraftMajors.map((m) => m.major))

  return (
    newestFirst(available.filter((m) => usedByMinecraft.has(m)))[0] ??
    newestFirst(available)[0]
  )
}

/** Majors for the major select, newest first. Without `showAll`, only the
 *  ones Minecraft uses. Without the catalog, those are all there is. */
export function visibleMajors(
  c: FEManagedJavaCatalog | undefined,
  minecraftMajors: FEMinecraftJavaMajor[],
  showAll: boolean
): number[] {
  const usedByMinecraft = new Set(minecraftMajors.map((m) => m.major))

  return availableMajors(c, minecraftMajors)
    .filter((m) => showAll || usedByMinecraft.has(m))
    .sort((a, b) => b - a)
}

/** The Minecraft versions a Java major serves. `getMinecraftJavaMajors` and
 *  the overview's groups both carry these fields. */
export type MinecraftRange = Pick<
  FEMinecraftJavaMajor,
  "major" | "fromVersion" | "toVersion" | "newest" | "oldest"
>

/** "1.20.5 – 1.21.11", "26.1 and newer", "1.16.5 and older" or "1.17".
 *  `undefined` when the version data is missing (offline, first start). */
export function minecraftRange(
  range: MinecraftRange,
  t: CatalogTranslate
): string | undefined {
  const { fromVersion: from, toVersion: to } = range

  if (range.newest) {
    return from ? t("java:_trn_minecraft_range_newer", { from }) : undefined
  }
  if (range.oldest) {
    return to ? t("java:_trn_minecraft_range_older", { to }) : undefined
  }
  if (!from || !to) return undefined
  return from === to ? from : t("java:_trn_minecraft_range", { from, to })
}

/** "Java 21 · Minecraft 1.20.5 – 1.21.11", with an open end for the newest
 *  and oldest majors. `undefined` without version data. */
export function minecraftRangeLabel(
  range: MinecraftRange,
  t: CatalogTranslate
): string | undefined {
  const versions = minecraftRange(range, t)
  if (versions === undefined) return undefined

  return t("java:_trn_major_minecraft", {
    major: range.major,
    range: versions
  })
}

export function majorLabel(
  major: number,
  lts: boolean,
  minecraftMajors: FEMinecraftJavaMajor[],
  t: CatalogTranslate
): string {
  const range = minecraftMajors.find((m) => m.major === major)
  return (
    (range && minecraftRangeLabel(range, t)) ??
    t(lts ? "java:_trn_major_lts" : "java:_trn_major", { major })
  )
}

export function filterBuilds(
  builds: FEManagedJavaBuild[],
  query: string
): FEManagedJavaBuild[] {
  const q = query.trim().toLowerCase()
  if (!q) return builds

  return builds.filter(
    (b) =>
      b.version.toLowerCase().includes(q) || b.name.toLowerCase().includes(q)
  )
}

export interface ProgressView {
  kind: "idle" | "download" | "extract" | "done"
  done: number
  total: number
  /** 0–1; `undefined` while the total isn't known. */
  fraction?: number
}

export function progressView(p: FEManagedJavaSetupProgress): ProgressView {
  if (p === "idle") return { kind: "idle", done: 0, total: 0 }
  if (p === "done") return { kind: "done", done: 0, total: 0, fraction: 1 }

  const [kind, [done, total]] =
    "downloading" in p
      ? (["download", p.downloading] as const)
      : (["extract", p.extracting] as const)
  const doneN = Number(done)
  const totalN = Number(total)

  return {
    kind,
    done: doneN,
    total: totalN,
    fraction: totalN > 0 ? Math.min(doneN / totalN, 1) : undefined
  }
}

/** "Downloading 12.4 MB of 42 MB" or "Extracting 120 of 300 files";
 *  `undefined` when there is nothing to count yet. */
export function progressLabel(
  view: ProgressView,
  t: CatalogTranslate,
  formatBytes: (bytes: number) => string
): string | undefined {
  if (view.total <= 0) return undefined

  switch (view.kind) {
    case "download":
      return t("java:_trn_install_downloading", {
        done: formatBytes(view.done),
        total: formatBytes(view.total)
      })
    case "extract":
      return t("java:_trn_install_extracting", {
        done: view.done,
        total: view.total
      })
    default:
      return undefined
  }
}

/** Numeric components with legacy `1.8.0_492` read as `8.0.492`, suffixes
 *  after `-` or `+` dropped, and trailing zeros trimmed (as the core does). */
export function versionKey(version: string): string | undefined {
  const core = version.split(/[-+]/)[0]
  const parts = core.split(/[._]/).map(Number)
  if (parts.some((p) => !Number.isInteger(p) || p < 0)) return undefined

  if (parts.length >= 3 && parts[0] === 1) parts.shift()
  while (parts.length > 0 && parts[parts.length - 1] === 0) parts.pop()

  return parts.join(".")
}

/** Numbers of a Java version: `1.8.0_492` is `8.0.492`, and anything from
 *  a `-` or `+` on is left out. */
function versionParts(version: string): number[] {
  const parts = version
    .split(/[-+]/)[0]
    .split(/[._]/)
    .map((part) => Number.parseInt(part, 10))
    .map((part) => (Number.isNaN(part) ? 0 : part))

  return parts.length >= 3 && parts[0] === 1 ? parts.slice(1) : parts
}

/** Orders Java versions numerically, part by part; a missing part is 0. */
export function compareJavaVersions(a: string, b: string): number {
  const left = versionParts(a)
  const right = versionParts(b)

  for (let i = 0; i < Math.max(left.length, right.length); i++) {
    const diff = (left[i] ?? 0) - (right[i] ?? 0)
    if (diff !== 0) return diff
  }
  return 0
}

/** The build the dialog offers first: the newest, or with `below` (Forge
 *  1.16.5 needs Java 8 below 8.0.312) the newest under it. `builds` is
 *  newest first. */
export function defaultBuild<B extends { version: string }>(
  builds: B[],
  below: string | undefined
): B | undefined {
  const fitting =
    below === undefined
      ? undefined
      : builds.find((b) => compareJavaVersions(b.version, below) < 0)
  return fitting ?? builds[0]
}

/** The folder directly under `managed_javas` that holds the Java. */
function installFolder(path: string): string | undefined {
  const parts = path.split(/[\\/]/)
  const at = parts.lastIndexOf("managed_javas")
  return at >= 0 ? parts[at + 1] : undefined
}

/** The distribution of a managed Java, from its install folder:
 *  `mojang-…` is Mojang and `zulu…` is Azul. */
function managedVendor(path: string): FEVendor | undefined {
  const folder = installFolder(path)
  if (folder?.startsWith("mojang-")) return "mojang"
  if (folder?.startsWith("zulu")) return "azul"
  return undefined
}

/** Azul's own name for a build, `zulu21.40.17` for
 *  `zulu21.40.17-ca-jre21.0.10-linux_x64.zip`. Azul publishes some versions
 *  twice, and this tells the two apart. */
function azulBuildName(name: string): string {
  return name.split("-")[0]
}

/** The Azul build a managed Java is, from its install folder; `undefined`
 *  for Mojang's, whose folders hold codenames. */
export function managedBuildName(path: string): string | undefined {
  const folder = installFolder(path)
  return folder && managedVendor(path) === "azul"
    ? azulBuildName(folder)
    : undefined
}

/** For catalog builds that share a version, Azul's build name for each, by
 *  build id. Mojang's build names are codenames, so they get none. */
export function buildHints(
  builds: FEManagedJavaBuild[],
  vendor: FEVendor
): Map<string, string> {
  const hints = new Map<string, string>()
  if (vendor !== "azul") return hints

  for (const build of builds) {
    const twins = builds.filter((b) => b.version === build.version)
    if (twins.length > 1) hints.set(build.id, azulBuildName(build.name))
  }
  return hints
}

/** The folder a Mojang build installs into, as the core names it. */
function mojangFolder(b: FEManagedJavaBuild): string {
  return `mojang-${b.name}-${b.version}`.replace(/[^A-Za-z0-9._-]/g, "_")
}

/** Whether the build is installed. A Mojang build is its own install folder,
 *  since the binary may report a shorter version than the one listed. Azul
 *  builds match a managed Azul Java by version. */
export function isBuildInstalled(
  b: FEManagedJavaBuild,
  javas: FEJavaComponent[],
  vendor: FEVendor
): boolean {
  const managed = javas.filter((j) => j.type === "managed")

  if (vendor === "mojang") {
    const folder = mojangFolder(b)
    return managed.some((j) => installFolder(j.path) === folder)
  }

  const wanted = versionKey(b.version)
  if (wanted === undefined) return false

  return managed.some(
    (j) => managedVendor(j.path) === vendor && versionKey(j.version) === wanted
  )
}

/** Distribution names are proper nouns and are not translated. */
export function vendorLabel(v: FEVendor): string {
  switch (v) {
    case "azul":
      return "Azul Zulu"
    case "mojang":
      return "Mojang"
    default: {
      const unhandled: never = v
      return unhandled
    }
  }
}

export function formatReleaseDate(
  iso: string | null,
  language: string
): string | undefined {
  if (!iso) return undefined
  const date = new Date(iso)
  if (Number.isNaN(date.getTime())) return undefined

  return new Intl.DateTimeFormat(intlLocale(language), {
    dateStyle: "medium"
  }).format(date)
}
