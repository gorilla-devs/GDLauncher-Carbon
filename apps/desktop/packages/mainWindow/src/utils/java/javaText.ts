import type { FEJavaEntry } from "@gd/core_module/bindings"
import { managedBuildName } from "../../managers/ModalsManager/modals/Java/managedJavaCatalog"
import { intlLocale } from "../intl"

// Text the Java settings, the Java dialogs and the instance pages share.

/** The slice of i18next's `t` `javaNeedText` needs, so tests can pass a
 *  stub. */
export type JavaNeedTranslate = (
  key:
    | "java:_trn_minecraft_needs_java"
    | "java:_trn_forge_needs_java_below"
    | "java:_trn_instance_needs_java_or_later",
  options: Record<string, string | number>
) => string

/** "Java4Shell and Log4Shell". */
export function knownIssueNames(names: string[], language: string): string {
  return new Intl.ListFormat(intlLocale(language), {
    type: "conjunction"
  }).format(new Set(names))
}

/** "Security flaw: Java4Shell and Log4Shell": the known-issue icon's
 *  tooltip. */
export function securityFlawText(
  names: string[],
  language: string,
  t: (
    key: "java:_trn_security_flaw",
    options: { name: string; interpolation: { escapeValue: boolean } }
  ) => string
): string {
  // Text, not HTML: it goes into tooltips and descriptions as it is.
  return t("java:_trn_security_flaw", {
    name: knownIssueNames(names, language),
    interpolation: { escapeValue: false }
  })
}

const ELLIPSIS = "…"

/** Shortens a path to `max` characters by dropping folders from the middle.
 *  It keeps the first folder (with the drive on Windows) and as many trailing
 *  parts as fit, and always the file name. */
export function middleTruncate(path: string, max: number): string {
  if (path.length <= max) return path

  const sep = /[\\/]/.exec(path)?.[0] ?? "/"
  const parts = path.split(sep)

  // Leading empty parts are the root ("/" or "\\server"), and a drive letter
  // goes with the folder after it.
  let headEnd = 0
  while (headEnd < parts.length - 1 && parts[headEnd] === "") headEnd++
  if (/^[A-Za-z]:$/.test(parts[headEnd]) && headEnd < parts.length - 2) {
    headEnd++
  }
  const head = parts.slice(0, headEnd + 1).join(sep)
  const shortened = (tailStart: number) =>
    [head, ELLIPSIS, ...parts.slice(tailStart)].join(sep)

  let tailStart = parts.length - 1
  while (tailStart - 1 > headEnd && shortened(tailStart - 1).length <= max) {
    tailStart--
  }

  // Nothing would be left out, so the ellipsis would only add length.
  if (tailStart <= headEnd + 1) return path

  return shortened(tailStart)
}

/** How many instance names a list shows before "and N more". */
const MAX_NAMES = 8

/** The first `MAX_NAMES` of `items`, and how many are left out. */
export function firstNames<T>(items: T[]): { shown: T[]; hidden: number } {
  const shown = items.slice(0, MAX_NAMES)
  return { shown, hidden: items.length - shown.length }
}

/** "Minecraft 1.21.4 needs Java 21", or with a cap on the build, which only
 *  Forge on 1.16.5 has: "Forge 1.16.5 needs Java 8 older than 8.0.312". With
 *  a Java `floor`, which a wrong-Java crash raised the instance's need to,
 *  "This instance needs Java 21 or later". */
export function javaNeedText(
  mcVersion: string,
  major: number,
  below: string | null,
  t: JavaNeedTranslate,
  floor: number | null = null
): string {
  if (floor !== null) {
    return t("java:_trn_instance_needs_java_or_later", { major: floor })
  }
  return below === null
    ? t("java:_trn_minecraft_needs_java", { version: mcVersion, major })
    : t("java:_trn_forge_needs_java_below", {
        version: mcVersion,
        major,
        below
      })
}

export type SizeUnitKey =
  | "java:_trn_unit_b"
  | "java:_trn_unit_kb"
  | "java:_trn_unit_mb"
  | "java:_trn_unit_gb"

/** Largest first, with the decimals each shows. */
const SIZE_UNITS: { key: SizeUnitKey; size: number; digits: number }[] = [
  { key: "java:_trn_unit_gb", size: 1024 ** 3, digits: 2 },
  { key: "java:_trn_unit_mb", size: 1024 ** 2, digits: 1 },
  { key: "java:_trn_unit_kb", size: 1024, digits: 1 },
  { key: "java:_trn_unit_b", size: 1, digits: 0 }
]

/** "189.4 MB", or "189,4 МБ" in Russian: binary units, as file managers
 *  count them, with the number in the app's language and the unit
 *  translated. */
export function formatSize(
  bytes: number,
  language: string,
  t: (key: SizeUnitKey) => string
): string {
  const safe = Number.isFinite(bytes) && bytes > 0 ? bytes : 0
  const unit =
    SIZE_UNITS.find((u) => safe >= u.size) ?? SIZE_UNITS[SIZE_UNITS.length - 1]

  const number = new Intl.NumberFormat(intlLocale(language), {
    minimumFractionDigits: unit.digits,
    maximumFractionDigits: unit.digits
  }).format(safe / unit.size)
  return `${number}\u00a0${t(unit.key)}`
}

/** Memory as the slider and its tooltip read it: "4 GB", "1.5 GB", or
 *  "512 MB" under a gigabyte, in the app's language. */
export function formatMemory(
  mb: number,
  language: string,
  t: (key: SizeUnitKey) => string
): string {
  const [value, unit]: [number, SizeUnitKey] =
    mb >= 1024 ? [mb / 1024, "java:_trn_unit_gb"] : [mb, "java:_trn_unit_mb"]
  const number = new Intl.NumberFormat(intlLocale(language), {
    maximumFractionDigits: 1
  }).format(value)
  return `${number}\u00a0${t(unit)}`
}

/** How far apart, as a share of the slider, two labels must sit to not run
 *  into each other. */
const MARK_GAP = 0.045

/** The memory slider's marks: 1 GB, 2 GB, 4 GB… up to `max` MB. A label
 *  too close to the one before keeps its tick but shows no text. */
export function memoryMarks(
  min: number,
  max: number,
  label: (mb: number) => string
): Record<number, string> {
  const marks: Record<number, string> = {}
  let lastShown: number | undefined

  for (let mb = min; mb <= max; mb *= 2) {
    const at = max > min ? (mb - min) / (max - min) : 0
    const fits = lastShown === undefined || at - lastShown >= MARK_GAP
    marks[mb] = fits ? label(mb) : ""
    if (fits) lastShown = at
  }

  return marks
}

/** Javas that read the same, "Azul Zulu 17.0.16" twice, each with what
 *  tells it apart: the folder an added or found Java lives in, or the Azul
 *  build of one of GDLauncher's. Mojang's folders hold codenames, so they
 *  get none. */
export function labelHints(
  javas: Pick<
    FEJavaEntry,
    "id" | "path" | "source" | "vendorName" | "version"
  >[]
): Map<string, string> {
  const label = (java: (typeof javas)[number]) =>
    `${java.vendorName} ${java.version}`
  const counts = new Map<string, number>()
  for (const java of javas) {
    counts.set(label(java), (counts.get(label(java)) ?? 0) + 1)
  }

  const hints = new Map<string, string>()
  for (const java of javas) {
    if ((counts.get(label(java)) ?? 0) < 2) continue
    const hint =
      java.source === "managed"
        ? managedBuildName(java.path)
        : javaHomeName(java.path)
    if (hint !== undefined) hints.set(java.id, hint)
  }
  return hints
}

/** The name of the folder a Java binary belongs to: `graalvm-21` for
 *  `/opt/graalvm-21/bin/java`, `zulu-21.jdk` for a macOS bundle. The whole
 *  path when that folder is a top-level one such as `/usr`, which names
 *  nothing. */
export function javaHomeName(path: string): string {
  const parts = path.split(/[\\/]/)
  let end = parts.length

  if (/^javaw?(\.exe)?$/i.test(parts[end - 1] ?? "")) end--
  if (end > 0 && parts[end - 1].toLowerCase() === "bin") end--
  if (end > 1 && parts[end - 1] === "Home" && parts[end - 2] === "Contents") {
    end -= 2
  }

  // The folders left before the home folder, past the root or drive.
  const above = parts
    .slice(0, end - 1)
    .filter((p) => p && !/^[A-Za-z]:$/.test(p))
  const name = parts[end - 1]

  return name && above.length > 0 ? name : path
}
