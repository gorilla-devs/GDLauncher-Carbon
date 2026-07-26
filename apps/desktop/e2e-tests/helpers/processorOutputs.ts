/**
 * Derives the set of library files a Forge/NeoForge install's client-side
 * processors must have produced, straight from the loader's own version
 * JSON — the same JSON the Rust core caches in `PartialVersionInfoCache` and
 * merges via `daedalus::modded::merge_partial_version` before running
 * `execute_processors` (`crates/carbon_app/src/managers/minecraft/forge.rs` /
 * `neoforge.rs`).
 *
 * This is a line-for-line port of
 * `crates/carbon_app/src/managers/minecraft/processor_outputs.rs`'s
 * `required_files` (plus the `GradleSpecifier` parsing/path-building it
 * depends on, from the `daedalus` crate's `lib.rs`), not a reimplementation
 * from the task description — the loader version installed is seeded-random
 * per run (see `helpers/instances.ts`'s `pickSeededOption`), so there is no
 * fixed set of paths to hardcode; the required set has to be computed from
 * whatever `processors`/`data` the actually-installed build's JSON contains.
 * `processorOutputs.test.ts` pins this port against the exact fixture JSON
 * the Rust module's own tests use, so a divergence between the two
 * implementations is caught in CI rather than discovered by this module
 * silently under- or over-asserting during a real install.
 *
 * Pure and I/O-free like `installVerify.ts`, so it is testable without a
 * Page or a live app.
 */

import path from "node:path"

/** Mirrors `daedalus::modded::Processor`. Field names are untouched by any
 *  `serde(rename_all)` on that struct, so this reads the cached JSON as-is. */
export interface Processor {
  jar: string
  classpath: string[]
  args: string[]
  outputs?: Record<string, string>
  sides?: string[]
}

/** Mirrors `daedalus::modded::SidedDataEntry`. */
export interface SidedDataEntry {
  client: string
  server: string
}

export interface RequiredLibrary {
  /** Relative to `<runtimePath>/libraries` — pass straight to
   *  `verifyLibrariesPresent`. */
  relativePath: string
  /** Known expected SHA-1 (lowercase hex), when the metadata declares one.
   *  Unused by this e2e suite (existence is the assertion that matters
   *  here), kept for parity with the Rust `RequiredFile` shape. */
  expectedSha1?: string
}

function isMavenRef(s: string): boolean {
  return s.startsWith("[") && s.endsWith("]")
}

/**
 * Finds `{KEY}` tokens, including ones embedded in longer strings such as
 * `{ROOT}/libraries/`. Mirrors `processor_outputs.rs`'s `data_keys_in`.
 */
function dataKeysIn(s: string): string[] {
  const keys: string[] = []
  let rest = s
  for (;;) {
    const start = rest.indexOf("{")
    if (start === -1) break
    const end = rest.indexOf("}", start + 1)
    if (end === -1) break
    keys.push(rest.slice(start + 1, end))
    rest = rest.slice(end + 1)
  }
  return keys
}

/**
 * SHA data entries are single-quoted 40-char hex literals, e.g.
 * `'de86b035d2da0f78940796bb95c39a932ed84834'`. Mirrors
 * `processor_outputs.rs`'s `strip_sha_literal`, including its use of
 * `trim_matches` (trims every leading/trailing quote, not just one).
 */
function stripShaLiteral(s: string): string | undefined {
  let start = 0
  let end = s.length
  while (start < end && s[start] === "'") start++
  while (end > start && s[end - 1] === "'") end--
  const trimmed = s.slice(start, end)
  if (trimmed.length === 40 && /^[0-9a-fA-F]+$/.test(trimmed)) {
    return trimmed.toLowerCase()
  }
  return undefined
}

/** Mirrors the side filter in `execute_processors`: no `sides` means every
 *  side, otherwise the list must contain "client". */
function runsOnClient(processor: Processor): boolean {
  return !processor.sides || processor.sides.includes("client")
}

interface GradleSpecifier {
  package: string
  artifact: string
  identifier?: string
  version: string
  extension: string
}

/**
 * Mirrors `daedalus`'s `impl FromStr for GradleSpecifier`
 * (`pkg:artifact:version[:identifier][-identifier...][@ext]`). Returns
 * `undefined` on anything that does not have at least package:artifact:version,
 * exactly like the Rust parser's `Err`.
 */
function parseGradleSpecifier(specifier: string): GradleSpecifier | undefined {
  const atParts = specifier.split("@")
  const namePart = atParts[0]
  const extension = atParts.length > 1 ? atParts[1] : "jar"
  if (extension === "") return undefined

  const nameItems = namePart.split(":")
  const pkg = nameItems[0]
  const artifact = nameItems[1]
  const version = nameItems[2]
  if (!pkg || artifact === undefined || version === undefined) return undefined

  const remaining = nameItems.slice(3)
  const identifier = remaining.length > 0 ? remaining.join("-") : undefined

  return { package: pkg, artifact, identifier, version, extension }
}

/** Mirrors `GradleSpecifier::filename`. */
function gradleSpecifierFilename(spec: GradleSpecifier): string {
  return spec.identifier
    ? `${spec.artifact}-${spec.version}-${spec.identifier}.${spec.extension}`
    : `${spec.artifact}-${spec.version}.${spec.extension}`
}

/** Mirrors `GradleSpecifier::into_path` — package dots become path segments. */
function gradleSpecifierToPath(spec: GradleSpecifier): string {
  return path.join(
    ...spec.package.split("."),
    spec.artifact,
    spec.version,
    gradleSpecifierFilename(spec)
  )
}

/**
 * Resolves a `[group:artifact:version...]`-bracketed maven reference to a
 * path relative to `libraries/`. Mirrors `processor_outputs.rs`'s
 * `resolve_ref`, minus the `libraries_path.join` (callers join
 * `libraries/<this>` themselves, e.g. via `verifyLibrariesPresent`).
 */
function resolveRef(mavenRef: string): string | undefined {
  if (!isMavenRef(mavenRef)) return undefined
  const inner = mavenRef.slice(1, -1)
  const spec = parseGradleSpecifier(inner)
  return spec ? gradleSpecifierToPath(spec) : undefined
}

function upsert(
  out: RequiredLibrary[],
  relativePath: string,
  expectedSha1: string | undefined
): void {
  const existing = out.find((f) => f.relativePath === relativePath)
  if (existing) {
    if (existing.expectedSha1 === undefined)
      existing.expectedSha1 = expectedSha1
  } else {
    out.push({ relativePath, expectedSha1 })
  }
}

/**
 * Port of `processor_outputs.rs`'s `required_files`. Walks every
 * client-side processor's args and declared outputs, resolving each maven
 * reference or `{DATA_KEY}` token against `data`, and returns the
 * deduplicated set of library paths (relative to `libraries/`) the install
 * must have produced.
 *
 * `data` absent (undefined) mirrors the Rust call sites passing `None` —
 * only direct maven-ref args/outputs resolve in that case, exactly like
 * `required_files(&procs, None, ...)`.
 */
export function requiredLibraryPaths(
  processors: Processor[],
  data?: Record<string, SidedDataEntry>
): RequiredLibrary[] {
  const out: RequiredLibrary[] = []

  const dataRefPath = (key: string): string | undefined => {
    const entry = data?.[key]
    return entry ? resolveRef(entry.client) : undefined
  }
  const dataSha = (key: string): string | undefined => {
    const entry = data?.[key]
    return entry ? stripShaLiteral(entry.client) : undefined
  }

  for (const processor of processors.filter(runsOnClient)) {
    for (const arg of processor.args) {
      if (isMavenRef(arg)) {
        const p = resolveRef(arg)
        if (p) upsert(out, p, undefined)
        continue
      }
      for (const key of dataKeysIn(arg)) {
        const p = dataRefPath(key)
        if (p) upsert(out, p, dataSha(`${key}_SHA`))
      }
    }

    if (processor.outputs) {
      for (const [outKey, outVal] of Object.entries(processor.outputs)) {
        let declaredSha: string | undefined
        if (isMavenRef(outVal)) {
          declaredSha = undefined
        } else {
          declaredSha = stripShaLiteral(outVal)
          if (declaredSha === undefined) {
            for (const key of dataKeysIn(outVal)) {
              const sha = dataSha(key)
              if (sha !== undefined) {
                declaredSha = sha
                break
              }
            }
          }
        }

        if (isMavenRef(outKey)) {
          const p = resolveRef(outKey)
          if (p) upsert(out, p, declaredSha)
        } else {
          for (const key of dataKeysIn(outKey)) {
            const p = dataRefPath(key)
            if (p) {
              const sha = declaredSha ?? dataSha(`${key}_SHA`)
              upsert(out, p, sha)
            }
          }
        }
      }
    }
  }

  return out
}
