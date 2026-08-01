/**
 * Reads an instance's own on-disk config file — the source of truth for its
 * name and Minecraft version/modloader selection, independent of both the
 * `Instance` DB row (which only carries `name`/`shortpath`, never the
 * version — see `versionCache.ts`'s `readInstanceByName`) and anything the
 * app's own UI/API reports. Pure Node: no Playwright, no DOM, so it can be
 * exercised directly against a synthetic file in a unit test, same as
 * `modVerify.ts`/`installVerify.ts`.
 *
 * Path and shape are taken from the Rust source, not guessed. The file lives
 * at `<instanceRoot>/instance.json` — directly at the instance root, sibling
 * to the `instance/` data subdirectory `mods/` lives under, not inside it —
 * per every real write/read site in `crates/carbon_app/src/managers/instance/mod.rs`:
 * `create_instance`, `update_instance` (×2), `duplicate_instance`, and the
 * startup `scan_instances`/`scan_instance` load path, five call sites, all
 * grepped individually rather than inferred from one. Deliberately **not**
 * `InstancePath::get_config_path()` (`crates/carbon_rt_path/src/lib.rs`,
 * `<instanceRoot>/instance/config`) — a same-sounding method name that names
 * a path nothing in the instance-metadata read/write path actually uses.
 *
 * The file is JSON whose current schema version is `v1`
 * (`crates/carbon_app/src/managers/instance/schema/v1.rs`). Field names on
 * the wire are the Rust struct's own snake_case names verbatim — neither
 * `v1::Instance` nor `v1::GameConfig` carries a `rename_all` — and
 * `GameVersion` is `#[serde(untagged)]`, so a standard (non-custom) version
 * serializes as `game_configuration.version` holding `{ release, modloaders }`
 * directly, with no wrapping `"Standard"` key; a custom version instead
 * serializes `game_configuration.version` as a bare string. The outer
 * `InstanceConfig` enum's `#[serde(tag = "_version")]` only adds a sibling
 * `"_version"` key at the top level — every `v1::Instance` field (including
 * `name` and `game_configuration`) is flattened alongside it, not nested
 * under a variant key, so this reads `name`/`game_configuration` straight off
 * the parsed top level.
 *
 * `modpack` (`v1::Instance.modpack: Option<ModpackInfo>`) is a second field
 * at that same top level, read the same way. Its shape is easy to get wrong
 * by analogy with `GameResolution` elsewhere in `v1.rs` (`#[serde(tag =
 * "type", content = "value")]`, a wrapped representation) — `Modpack` instead
 * carries plain `#[serde(tag = "platform")]`, **no `content`**, on a newtype
 * variant wrapping a struct (`Curseforge(CurseforgeModpack)` /
 * `Modrinth(ModrinthModpack)`), which is the internally-tagged form serde
 * merges flat: the tag sits alongside the variant's own fields, not wrapping
 * them under a `"value"` key. `ModpackInfo` then `#[serde(flatten)]`s that
 * `Modpack` together with its own `locked: bool`. So on disk this is one flat
 * object — `{ "platform": "Modrinth", "project_id": "...", "version_id":
 * "...", "locked": true }` (CurseForge: `"platform": "Curseforge"`,
 * `"project_id"`/`"file_id"` as JSON numbers, `u32` on the Rust side) — never
 * `{ modpack: { platform, value: { ... } }, locked }`. Confirmed both by
 * reading `v1.rs` directly and against a live installed instance's
 * `instance.json` (see `modpackInstall.spec.ts`).
 */

import fs from "node:fs/promises"
import path from "node:path"

export interface OnDiskModLoader {
  type: string
  version: string
}

/** `v1::ModpackInfo` flattened with `v1::Modpack`, read off `instance.json`'s
 *  `modpack` key — see this file's module doc comment for the exact wire
 *  shape and why it is not the `{ modpack: { platform, value } }` nesting a
 *  naive read of `v1.rs`'s sibling `GameResolution` enum would suggest.
 *  Platform-specific fields follow `helpers/mods.ts`'s `InstalledMod`
 *  convention (paired `modrinthX`/`curseforgeX` fields, the inapplicable pair
 *  `null`) rather than a single polymorphic field, for the same reason: the
 *  two platforms' id types genuinely differ (Modrinth's are strings,
 *  CurseForge's are `u32`s), so keeping them separate avoids a lossy coercion
 *  at the read site. */
export interface OnDiskModpackInfo {
  platform: "modrinth" | "curseforge"
  /** Set only when `platform === "modrinth"`. */
  modrinthProjectId: string | null
  /** Set only when `platform === "modrinth"` — the specific version the
   *  instance is pinned to (`ModrinthModpack.version_id`). */
  modrinthVersionId: string | null
  /** Set only when `platform === "curseforge"`. */
  curseforgeProjectId: number | null
  /** Set only when `platform === "curseforge"` — the specific file the
   *  instance is pinned to (`CurseforgeModpack.file_id`). */
  curseforgeFileId: number | null
  /** One-way once set: `unlockModpack` (`helpers/modpacks.ts`) can flip this
   *  to `false`, but the shipped UI has no control to flip it back. */
  locked: boolean
}

export interface OnDiskInstanceConfig {
  name: string
  /** `game_configuration.version.release` — `null` when the instance has a
   *  `Custom` version string instead of a `Standard` one, or no version set
   *  at all (both are legitimate states on the Rust side, `Option<GameVersion>`
   *  and the untagged `Custom(String)` variant). */
  mcVersion: string | null
  modloaders: OnDiskModLoader[]
  /** `seconds_played` — the launcher's own running total for this instance,
   *  rewritten by `update_playtime` (`managers/instance/mod.rs`) on every
   *  bank. `#[serde(default)]` on the Rust side, so an instance that has
   *  never been played may omit the key entirely; read as 0 in that case
   *  rather than treated as a malformed file. */
  secondsPlayed: number
  /** `null` for a plain (non-modpack) instance — `v1::Instance.modpack` is
   *  `Option<ModpackInfo>` and `#[serde(default)]`, so the key may also be
   *  entirely absent, treated identically to an explicit `null`. */
  modpack: OnDiskModpackInfo | null
}

interface RawStandardVersion {
  release?: string
  modloaders?: OnDiskModLoader[]
}

/** Raw wire shape of `instance.json`'s `modpack` key — see this file's module
 *  doc comment. `project_id` is typed as the union of both platforms' wire
 *  types (Modrinth: JSON string; CurseForge: JSON number, `u32` on the Rust
 *  side) since which one is present depends on the sibling `platform` field,
 *  not on this field alone. */
interface RawModpackInfo {
  platform?: "Curseforge" | "Modrinth"
  project_id?: string | number
  version_id?: string
  file_id?: number
  locked?: boolean
}

interface RawV1Config {
  name?: string
  seconds_played?: number
  game_configuration?: {
    version?: RawStandardVersion | string | null
  }
  modpack?: RawModpackInfo | null
}

/**
 * Maps `RawModpackInfo` (the wire shape) to `OnDiskModpackInfo` (this
 * module's own shape — see its doc comment for why the two platforms get
 * separate, paired fields). `configPath` is only for the thrown message.
 *
 * Throws on a `platform` value that is neither `"Curseforge"` nor
 * `"Modrinth"` rather than silently mapping it to one or the other —
 * `readInstanceConfig`'s own "never a vacuous result" stance (see its doc
 * comment) applies just as much to a field that parses into the *wrong*
 * platform silently as it does to the file being unreadable outright: a
 * caller comparing `platform === "modrinth"` would otherwise just read
 * `false` for a genuine schema drift instead of getting a named failure.
 */
function parseModpack(
  raw: RawModpackInfo | null | undefined,
  configPath: string
): OnDiskModpackInfo | null {
  if (!raw) return null

  if (raw.platform === "Modrinth") {
    return {
      platform: "modrinth",
      modrinthProjectId:
        typeof raw.project_id === "string" ? raw.project_id : null,
      modrinthVersionId: raw.version_id ?? null,
      curseforgeProjectId: null,
      curseforgeFileId: null,
      locked: raw.locked ?? false
    }
  }
  if (raw.platform === "Curseforge") {
    return {
      platform: "curseforge",
      modrinthProjectId: null,
      modrinthVersionId: null,
      curseforgeProjectId:
        typeof raw.project_id === "number" ? raw.project_id : null,
      curseforgeFileId: raw.file_id ?? null,
      locked: raw.locked ?? false
    }
  }

  throw new Error(
    `readInstanceConfig: ${configPath} has a "modpack" object with an ` +
      `unrecognised "platform" (got ${JSON.stringify(raw.platform)}) — ` +
      'expected "Curseforge" or "Modrinth"'
  )
}

/**
 * Reads and parses `<instanceRoot>/instance.json`. Throws — never a vacuous
 * result — on a missing or malformed file: unlike `modVerify.ts`'s
 * never-throw contract for `mods/` (legitimately absent until the first
 * install), there is no legitimate "not there yet" state for this file once
 * instance creation has returned — it is written synchronously as part of
 * creation (`managers/instance/mod.rs`'s `create_instance`), not lazily.
 */
export async function readInstanceConfig(
  instanceRoot: string
): Promise<OnDiskInstanceConfig> {
  const configPath = path.join(instanceRoot, "instance.json")

  let raw: string
  try {
    raw = await fs.readFile(configPath, "utf8")
  } catch (error) {
    throw new Error(
      `readInstanceConfig: could not read ${configPath}: ` +
        `${(error as Error).message}`
    )
  }

  let parsed: RawV1Config
  try {
    parsed = JSON.parse(raw) as RawV1Config
  } catch (error) {
    throw new Error(
      `readInstanceConfig: ${configPath} is not valid JSON: ` +
        `${(error as Error).message}`
    )
  }

  if (typeof parsed.name !== "string") {
    throw new Error(
      `readInstanceConfig: ${configPath} has no string "name" field ` +
        `(got ${JSON.stringify(parsed.name)})`
    )
  }

  const version = parsed.game_configuration?.version
  const isStandard =
    typeof version === "object" && version !== null && "release" in version

  return {
    name: parsed.name,
    mcVersion: isStandard ? (version.release ?? null) : null,
    modloaders: isStandard ? (version.modloaders ?? []) : [],
    secondsPlayed:
      typeof parsed.seconds_played === "number" ? parsed.seconds_played : 0,
    modpack: parseModpack(parsed.modpack, configPath)
  }
}
