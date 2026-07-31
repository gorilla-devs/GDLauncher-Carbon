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
 */

import fs from "node:fs/promises"
import path from "node:path"

export interface OnDiskModLoader {
  type: string
  version: string
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
}

interface RawStandardVersion {
  release?: string
  modloaders?: OnDiskModLoader[]
}

interface RawV1Config {
  name?: string
  seconds_played?: number
  game_configuration?: {
    version?: RawStandardVersion | string | null
  }
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
      typeof parsed.seconds_played === "number" ? parsed.seconds_played : 0
  }
}
