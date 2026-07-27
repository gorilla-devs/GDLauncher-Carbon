/**
 * Reads the loader/vanilla version JSON the app already fetched and cached in
 * the runtime's own `gdl_conf.db`, straight off disk — never re-fetched from
 * the network, so verification never depends on a second source that could
 * disagree with what actually got installed for reasons unrelated to the
 * install itself. Both `instanceInstall.spec.ts` (vanilla matrix) and
 * `loaderInstall.spec.ts` (loader matrix, both its vanilla-substrate check
 * and its loader-specific one) read through this module rather than each
 * carrying its own copy of the table/column/blob-decode contract, so a
 * `carbon_repos` rename only needs to change in one place.
 *
 * The core does not write these as loose files under the runtime path; it
 * caches the exact response bytes it fetched as a blob column, one row per
 * cache key (see `get_version`'s `version_meta::upsert_version_info` /
 * `upsert_partial_version_info` calls in
 * `crates/carbon_app/src/managers/minecraft/minecraft.rs` / `forge.rs` /
 * `neoforge.rs` / `fabric.rs` / `quilt.rs`, and the table shapes in
 * `crates/carbon_repos/src/repos/version_meta.rs`). That db is opened
 * WAL-mode by the core (`crates/carbon_repos/src/db_exec.rs`), so a separate
 * read-only connection from here is safe to open concurrently.
 */

import { DatabaseSync } from "node:sqlite"
import path from "node:path"
import type { Processor, SidedDataEntry } from "./processorOutputs.js"

/**
 * Opens `runtimePath`'s `gdl_conf.db` read-only for the duration of `fn`,
 * closing it afterward regardless of outcome. The one place this suite opens
 * that database, so every reader against it — the blob-cache reader below,
 * and `readInstanceByName`'s plain-column read further down — shares one
 * connection-lifecycle implementation instead of each hand-rolling its own
 * open/close pair. This module's own history is why that matters: three
 * separate copies of the version-cache DB read existed before being
 * collapsed into this file (see `f3c8af8`'s commit message), and a shortpath
 * lookup is not reason enough to start a fourth.
 *
 * Exported so `dbSeed.test.ts` can read back a seeded database's structure
 * through the same connection lifecycle rather than hand-rolling a fifth.
 */
export function withConfigDb<T>(
  runtimePath: string,
  fn: (db: DatabaseSync) => T
): T {
  const dbPath = path.join(runtimePath, "gdl_conf.db")
  const db = new DatabaseSync(dbPath, { readOnly: true })
  try {
    return fn(db)
  } finally {
    db.close()
  }
}

/**
 * Reads `column` from the row keyed by `id` in `table`, and JSON-parses it as
 * UTF-8. Every cache table this suite reads follows the same shape (one blob
 * column, keyed by a string id), so this is the one place that shape
 * assumption — and its failure modes (no such row; the column isn't a blob)
 * — is expressed, rather than each caller re-deriving its own error wording
 * for the same two ways a cache read can be wrong.
 */
function readBlobRow(
  runtimePath: string,
  table: string,
  column: string,
  id: string,
  notFoundHint: string
): unknown {
  return withConfigDb(runtimePath, (db) => {
    const row = db
      .prepare(`SELECT ${column} FROM ${table} WHERE id = ?`)
      .get(id) as Record<string, unknown> | undefined

    if (!row) {
      throw new Error(
        `no cached row for "${id}" in ${path.join(runtimePath, "gdl_conf.db")} ` +
          `(table ${table}) — ${notFoundHint}`
      )
    }

    const blob = row[column]
    if (!(blob instanceof Uint8Array)) {
      throw new Error(
        `${table}.${column} for "${id}" in ${path.join(runtimePath, "gdl_conf.db")} ` +
          `is not a blob (got ${typeof blob}) — cache row is malformed`
      )
    }

    return JSON.parse(Buffer.from(blob).toString("utf8"))
  })
}

/** The subset of Mojang's version JSON this suite needs off of it. */
export interface CachedVersionInfo {
  assetIndex?: { id?: string }
  downloads?: { client?: { sha1?: string } }
}

/**
 * Reads the app's cached *vanilla* version JSON for `mcVersion` off
 * `VersionInfoCache`.
 *
 * `assetIndex.id` (not the sibling `assets` string field — same value on a
 * well-formed manifest, but `assetIndex.id` is what the core actually names
 * the cached index file after, in `assets.rs`'s `get_assets_dir`) is the
 * asset index id for `verifyAssetIndex`: it is never the version id (e.g.
 * live Mojang data has 1.20.1 sharing bare numeric id `"5"` with a run of
 * other releases, and 1.12.2/1.16.5 sharing the minor-only `"1.12"`/`"1.16"`),
 * so this always reads the real value off the cached JSON rather than
 * assuming one. (The id literally spelled `legacy` — the one real
 * `"virtual": true` index left in live Mojang data, verified directly
 * against it — belongs to the 1.6.x release line, not 1.7.10; that is why
 * `versionMatrix.ts`'s `PINNED_VERSIONS` pins `1.6.4` specifically to
 * exercise `verifyAssetIndex`'s virtual branch. That branch itself reads the
 * index JSON's own `"virtual"` field, never the id string or the version id,
 * so nothing here depends on which id Mojang currently routes through it.)
 *
 * Keyed by the Minecraft version, never by loader or loader version: every
 * loader combination installs onto a real vanilla Minecraft version
 * underneath, and `daedalus::modded::merge_partial_version` always takes
 * `asset_index` and (via `inherits_from`) the client jar's location from the
 * *vanilla* `VersionInfo` passed in, never from the loader's own partial one
 * (confirmed in `crates/carbon_app/src/managers/instance/run/minecraft.rs`,
 * which resolves `client_path` off `version_info.inherits_from` after
 * merging, and in daedalus's `merge_partial_version` itself, which sets
 * `asset_index: merge.asset_index` unconditionally).
 */
export function readVersionInfo(
  runtimePath: string,
  mcVersion: string
): CachedVersionInfo {
  return readBlobRow(
    runtimePath,
    "VersionInfoCache",
    "versionInfo",
    mcVersion,
    "the app never downloaded it, or the cache key does not match the version id"
  ) as CachedVersionInfo
}

/** Mirrors `daedalus::minecraft::Library`'s own field this suite needs off
 *  it — the maven coordinate a Fabric/Quilt loader-version JSON declares
 *  itself (and its dependencies) under. */
export interface CachedLoaderLibrary {
  name: string
}

/** The subset of `daedalus::modded::PartialVersionInfo` this suite needs off
 *  a Forge/NeoForge/Fabric/Quilt cache entry. */
export interface CachedPartialVersionInfo {
  processors?: Processor[]
  data?: Record<string, SidedDataEntry>
  libraries?: CachedLoaderLibrary[]
}

/**
 * Reads the loader-version JSON the app already fetched and cached for
 * `cacheId`, straight off `PartialVersionInfoCache`
 * (`crates/carbon_app/src/managers/minecraft/forge.rs` / `neoforge.rs` /
 * `fabric.rs` / `quilt.rs`'s `get_version`). `cacheId` is
 * `"<loader>-<version>"` — the exact `db_entry_name` each of those modules
 * builds from the chosen loader version string.
 *
 * This is what makes `loaderInstall.spec.ts`'s disk-verification assertions
 * possible without hardcoding any maven paths: `processors`/`data`/
 * `libraries` are whatever the *actually-installed, seeded-random* build
 * declared, read back rather than assumed.
 */
export function readPartialVersionInfo(
  runtimePath: string,
  cacheId: string
): CachedPartialVersionInfo {
  return readBlobRow(
    runtimePath,
    "PartialVersionInfoCache",
    "partialVersionInfo",
    cacheId,
    "the app never fetched it, or the cache key does not match the loader version string"
  ) as CachedPartialVersionInfo
}

/** The subset of `carbon_repos::repos::instance::InstanceRow` the e2e suite
 *  needs off it: `id` is the numeric `FEInstanceId` the rspc API takes for
 *  every per-instance call (e.g. `InstallMod.instance_id`,
 *  `packages/core_module/bindings.d.ts`); `shortpath` is the instance's
 *  actual on-disk directory name under `<runtimePath>/instances/`. */
export interface InstanceRow {
  id: number
  shortpath: string
}

/**
 * Reads an instance's database row by its display `name`, straight off the
 * `Instance` table in `gdl_conf.db` — never assumed off the name itself.
 *
 * Neither field this suite needs is recoverable from the DOM: the library
 * tile only ever renders `data-instance-name`/`data-instance-state`/
 * `data-instance-failed` (`components/Instance/Tile.tsx`), never the row id
 * or the shortpath. The shortpath in particular is not the display name —
 * `next_folder` (`crates/carbon_app/src/managers/instance/mod.rs`) derives it
 * from the name via `sanitize_name` plus a numeric-suffix dedup loop against
 * whatever already exists on disk, so a name containing characters
 * `sanitize_name` replaces, or colliding with an existing instance's folder,
 * gets a shortpath that genuinely differs from the name typed in the
 * creation modal. `InstancesPath::get_instance_path(shortpath)`
 * (`crates/carbon_rt_path/src/lib.rs`) is what actually resolves an
 * instance's on-disk root from it.
 *
 * `name` is assumed unique here — nothing in `Instance`'s schema enforces
 * that, but every caller in this suite creates its own uniquely-named
 * instance, so a duplicate is itself a test bug worth surfacing. Enforced
 * directly: this reads every matching row and throws if more than one comes
 * back, rather than silently picking the first the way a plain `.get()`
 * would.
 */
export function readInstanceByName(
  runtimePath: string,
  name: string
): InstanceRow {
  return withConfigDb(runtimePath, (db) => {
    const rows = db
      .prepare(`SELECT id, shortpath FROM Instance WHERE name = ?`)
      .all(name) as { id: number; shortpath: string }[]

    if (rows.length === 0) {
      throw new Error(
        `no Instance row named "${name}" in ` +
          `${path.join(runtimePath, "gdl_conf.db")} — was it actually ` +
          "created, and does the name match exactly?"
      )
    }
    if (rows.length > 1) {
      throw new Error(
        `${rows.length} Instance rows named "${name}" in ` +
          `${path.join(runtimePath, "gdl_conf.db")} — every caller in this ` +
          "suite creates its own uniquely-named instance, so a duplicate " +
          "is a test bug worth surfacing, not something to silently pick " +
          "one of"
      )
    }

    const [row] = rows
    return { id: row.id, shortpath: row.shortpath }
  })
}

/** The subset of the single-row `AppConfiguration` table (the app-wide
 *  settings row) `persistence.spec.ts` needs off it. */
export interface AppConfigurationRow {
  reducedMotion: boolean
}

/**
 * Reads the single-row `AppConfiguration` table straight off `gdl_conf.db` —
 * the disk-side half of `persistence.spec.ts`'s "an app setting survives a
 * restart" case, mirroring `readInstanceByName`'s plain-column read directly
 * above. Unlike `readVersionInfo`/`readPartialVersionInfo`, this is not a
 * JSON-blob cache: `settings.setSettings` writes straight into typed columns
 * on this table (`crates/carbon_repos/src/repos/app_configuration.rs`), so
 * this is a direct column read rather than a `readBlobRow` call.
 *
 * `AppConfiguration` always has exactly one row, at `id = 0` (seeded by the
 * baseline/migration chain) — this throws rather than returning a default if
 * that row is somehow missing, the same "never silently substitute" stance
 * `resolveLoaderVersionSeed` takes above.
 */
export function readAppConfiguration(runtimePath: string): AppConfigurationRow {
  return withConfigDb(runtimePath, (db) => {
    const row = db
      .prepare(`SELECT reducedMotion FROM AppConfiguration WHERE id = 0`)
      .get() as { reducedMotion: number | bigint } | undefined

    if (!row) {
      throw new Error(
        `no AppConfiguration row (id=0) in ` +
          `${path.join(runtimePath, "gdl_conf.db")} — this table should ` +
          "always have exactly one row"
      )
    }

    return { reducedMotion: Boolean(row.reducedMotion) }
  })
}
