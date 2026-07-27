/**
 * Plants `<runtimePath>/gdl_conf.db` in one of the terminal states the
 * bidirectional migration runner (`crates/carbon_repos/src/compat.rs`) can
 * discover on startup, so a recovery-screen e2e test can point a real app
 * launch at a fresh runtime dir and observe the exact `_STATUS_:<EVENT>` line
 * the corresponding real-world condition produces
 * (`crates/carbon_app/src/managers/db_bootstrap.rs`'s `DbStatus` funnel,
 * consumed by `apps/desktop/packages/main/index.ts`'s core-process handler).
 *
 * Every seed here is built to satisfy the *actual* branch conditions in
 * `compat.rs`'s `MigrationSet::open`/`handle_ahead`/`down_run` — not to
 * imitate the symptom by another route. Two states could not be produced
 * honestly and are not offered; see the `SeedState` doc comment.
 *
 * WAL mode (spec: `crates/carbon_rt_path/src/lib.rs`'s runtime layout,
 * `gdl_conf.db` at the root) means a stale `-wal`/`-shm` sidecar beside a
 * freshly written main file can silently resurrect old pages the moment the
 * core opens it. Every seed here opens with `PRAGMA journal_mode = DELETE`
 * (see `openWritable`) so every write lands directly in the main file with no
 * sidecar ever created, and `resetRuntimeDb` removes any sidecar left by a
 * previous run before writing anything new.
 */

import { DatabaseSync } from "node:sqlite"
import { createHash } from "node:crypto"
import fs from "node:fs"
import path from "node:path"
import { fileURLToPath } from "node:url"

const __dirname = path.dirname(fileURLToPath(import.meta.url))
// helpers -> e2e-tests -> desktop -> apps -> repo root.
const REPO_ROOT = path.resolve(__dirname, "..", "..", "..", "..")
const MIGRATIONS_DIR = path.join(
  REPO_ROOT,
  "crates/carbon_repos/prisma/migrations"
)
const BASELINE_SQL_PATH = path.join(
  REPO_ROOT,
  "crates/carbon_repos/baseline/baseline.sql"
)
const FIRST_MIGRATION_NAME = "20240120134904_init"
const FIRST_MIGRATION_SQL_PATH = path.join(
  MIGRATIONS_DIR,
  FIRST_MIGRATION_NAME,
  "migration.sql"
)

/**
 * The six `_STATUS_:` events `db_bootstrap.rs`'s `DbStatus` formatter can
 * emit for a fatal or informational DB outcome, minus one this helper cannot
 * honestly produce:
 *
 * - `DB_DOWNGRADED` is INCLUDED, via a synthesized `_migrations` tail (see
 *   `seedDbDowngraded`) — not the "additive-only tail" the task brief
 *   describes. Reading `compat.rs` directly (`OpenVerdict::Downgraded`'s own
 *   doc comment, and `handle_ahead`'s `if ahead.iter().all(...Additive)`
 *   branch) shows an all-additive tail overlays and returns `Proceed`
 *   *silently* — no status line at all. `Downgraded` only fires after a
 *   *breaking* tail's stored `down_sql` runs successfully and its result
 *   matches this binary's own schema byte-for-byte. "Additive" alone does
 *   not decide the classification; the code above does.
 * - `DB_MIGRATION_FAILED` is producible (see `seedDbMigrationFailed`) via a
 *   genuine forward-apply collision, not a symptom substitution.
 *
 * Not offered: nothing. All six of `db_bootstrap.rs`'s emittable statuses
 * turned out to be reachable by a seed that drives the real branch condition
 * — see each `seed*` function's doc comment for exactly which one.
 */
export type SeedState =
  | "DB_CORRUPT"
  | "BACKWARDS_MIGRATION"
  | "DB_DIVERGED"
  | "DB_DOWNGRADE_FAILED"
  | "DB_DOWNGRADED"
  | "DB_MIGRATION_FAILED"

/**
 * Verbatim copy of `compat.rs`'s `CREATE_MIGRATIONS_TABLE`. Duplicated
 * (rather than shelling out to Rust to get it) because the runner itself
 * only ever runs this as `CREATE TABLE IF NOT EXISTS`, so a mismatch here
 * would surface as a column-shape error the moment the real binary's
 * `ensure_migrations_table` call is a no-op against a table we already
 * created with the wrong shape — loud, not silent. Keep in sync with
 * `crates/carbon_repos/src/compat.rs`'s `CREATE_MIGRATIONS_TABLE` if that
 * schema ever changes.
 */
const CREATE_MIGRATIONS_TABLE = `CREATE TABLE IF NOT EXISTS _migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    checksum TEXT NOT NULL,
    kind TEXT NOT NULL,
    down_sql TEXT,
    data_down TEXT NOT NULL DEFAULT 'full',
    applied_at INTEGER NOT NULL)`

const INSERT_MIGRATION_ROW = `INSERT INTO _migrations
   (version, name, checksum, kind, down_sql, data_down, applied_at)
   VALUES (?, ?, ?, ?, ?, ?, ?)`

interface MigrationRow {
  version: number
  name: string
  checksum: string
  kind: "additive" | "breaking"
  downSql: string | null
  dataDown: string
  appliedAt: number
}

function insertMigrationRow(db: DatabaseSync, row: MigrationRow): void {
  db.prepare(INSERT_MIGRATION_ROW).run(
    row.version,
    row.name,
    row.checksum,
    row.kind,
    row.downSql,
    row.dataDown,
    row.appliedAt
  )
}

/**
 * The number of migration directories this checkout carries — `crates/
 * carbon_repos/src/lib.rs`'s `get_migrations()` is the single source of
 * truth binding directory -> MigrationDef, one
 * `historical_migration!(...)` entry per directory
 * (confirmed by hand: 25 directories under `prisma/migrations/` excluding
 * `README.md`, 25 `historical_migration!` invocations in `get_migrations()`,
 * at the time this was written). Counting directories rather than hardcoding
 * 25 keeps `DB_DOWNGRADE_FAILED`/`DB_DOWNGRADED`'s "one version past this
 * binary's own count" seed correct as new migrations are added, since the
 * app under test is always built from this same checkout.
 */
export function migrationCount(): number {
  return fs
    .readdirSync(MIGRATIONS_DIR, { withFileTypes: true })
    .filter((entry) => entry.isDirectory()).length
}

/** sha256 (hex), matching `compat.rs`'s `sha256_hex` exactly (sha256 then
 *  lowercase-hex encode) — the same algorithm `MigrationSet::checksum` hashes
 *  a migration's raw `up_sql` bytes with. */
function sha256Hex(data: Buffer | string): string {
  return createHash("sha256").update(data).digest("hex")
}

/**
 * The real checksum the running binary computes for migration 1
 * (`20240120134904_init`) — `sha256_hex(up_sql.as_bytes())` in
 * `MigrationSet::checksum`, where `up_sql` is `include_str!`'d from this
 * exact file verbatim (no trimming), so this hashes the raw file bytes
 * rather than a re-encoded string to avoid any encoding round-trip risk.
 * Exported so `dbSeed.test.ts` can assert the diverged seed's stored
 * checksum is genuinely wrong, not just "some string".
 */
export function firstMigrationRealChecksum(): string {
  return sha256Hex(fs.readFileSync(FIRST_MIGRATION_SQL_PATH))
}

function dbPaths(runtimePath: string) {
  const main = path.join(runtimePath, "gdl_conf.db")
  return {
    main,
    wal: `${main}-wal`,
    shm: `${main}-shm`,
    // `snapshot_path_for` in both `compat.rs` and `db_bootstrap.rs`:
    // `<stem>.pre-downgrade.db` beside the main file.
    snapshot: path.join(runtimePath, "gdl_conf.pre-downgrade.db")
  }
}

/**
 * Clears every file a previous seed or a previous app run could have left at
 * `runtimePath` for `gdl_conf.db` — the main file, its WAL sidecars, and a
 * stray pre-downgrade snapshot — then ensures the directory exists. Every
 * `seed*` function starts from this so two seeds run back-to-back against the
 * same runtime dir (or a leftover app run) can never leak state between them.
 */
function resetRuntimeDb(runtimePath: string): void {
  fs.mkdirSync(runtimePath, { recursive: true })
  const { main, wal, shm, snapshot } = dbPaths(runtimePath)
  for (const f of [main, wal, shm, snapshot]) {
    fs.rmSync(f, { force: true })
  }
}

/**
 * Opens `dbFile` read-write and forces rollback-journal mode so every write
 * this module makes lands directly in the main file — no `-wal`/`-shm`
 * sidecar is ever created by seeding, only ever by the app itself opening the
 * result in WAL mode (`db_bootstrap.rs`'s `migrate_db`).
 */
function openWritable(dbFile: string): DatabaseSync {
  const db = new DatabaseSync(dbFile)
  db.exec("PRAGMA journal_mode = DELETE;")
  return db
}

// ---------------------------------------------------------------------------
// DB_CORRUPT
// ---------------------------------------------------------------------------

/**
 * A file that is not a SQLite database at all. `db_bootstrap.rs`'s own
 * `corrupt_db_file_reports_corrupt_status` test uses this exact byte string
 * against a real `load_and_migrate` call to prove `NotADatabase` funnels to
 * `DB_CORRUPT` (`is_corruption` in `db_bootstrap.rs`); reused verbatim here
 * rather than re-deriving an equivalent fixture.
 */
function seedDbCorrupt(runtimePath: string): void {
  const { main } = dbPaths(runtimePath)
  fs.writeFileSync(
    main,
    Buffer.from("this is definitely not a sqlite database file")
  )
}

// ---------------------------------------------------------------------------
// BACKWARDS_MIGRATION
// ---------------------------------------------------------------------------

/**
 * `user_version` far ahead of any real migration count, on a database whose
 * `_migrations` table has never been created — the "metadata missing above
 * count" branch in `compat.rs`'s `handle_ahead` (`CENSUS-RULE:
 * compat.backwards-missing-metadata`): every version in `(count+1..=
 * user_version]` must have a row, and an empty/absent table satisfies "at
 * least one is missing" trivially, for any real `count`.
 *
 * Deliberately does NOT pre-create `_migrations` itself: `MigrationSet::
 * open` calls `ensure_migrations_table` (`CREATE TABLE IF NOT EXISTS`)
 * unconditionally before checking anything else, so the real binary creates
 * it empty — which is exactly the "no metadata" condition this state needs.
 * Hand-rolling that schema here would only risk drifting from it for no
 * benefit. A throwaway marker table is created instead, solely so
 * `sqlite_master` is non-empty: `MigrationSet::open` checks
 * `sqlite_master_is_empty` FIRST and would otherwise take the fresh-install
 * baseline path (`install_baseline`), which always succeeds and never
 * reaches this branch at all.
 */
function seedBackwardsMigration(runtimePath: string): void {
  const { main } = dbPaths(runtimePath)
  const db = openWritable(main)
  try {
    db.exec(`CREATE TABLE __seed_marker (id INTEGER PRIMARY KEY);`)
    // Ahead of any plausible real count (25 today) by 1,000,000, so this
    // keeps holding as migrations are added without needing to reason about
    // the exact current count.
    const version = migrationCount() + 1_000_000
    db.exec(`PRAGMA user_version = ${version};`)
  } finally {
    db.close()
  }
}

// ---------------------------------------------------------------------------
// DB_DIVERGED
// ---------------------------------------------------------------------------

/** Deliberately wrong 64-hex-char checksum for the diverged seed. Checked
 *  against the real one at seed time (see `seedDbDiverged`) rather than
 *  trusted to differ by construction. */
const DIVERGED_WRONG_CHECKSUM = "0".repeat(64)

/**
 * A `_migrations` row for version 1 whose stored checksum does not match
 * what this binary computes for `20240120134904_init`'s real `up_sql` —
 * `compat.rs`'s `first_divergent`, which `MigrationSet::open` checks before
 * anything else (`CENSUS-RULE: compat.diverged-checksum`), for every version
 * `1..=min(user_version, count)`. `user_version = 1` puts the check
 * unavoidably in range regardless of this binary's real migration count.
 */
function seedDbDiverged(runtimePath: string): void {
  const { main } = dbPaths(runtimePath)
  const real = firstMigrationRealChecksum()
  if (DIVERGED_WRONG_CHECKSUM === real) {
    // Cosmically unlikely, but a seed that accidentally matches the real
    // checksum would silently stop being a divergence at all — fail loud
    // rather than let that pass unnoticed.
    throw new Error(
      "diverged seed's sentinel checksum accidentally matches the real one"
    )
  }
  const db = openWritable(main)
  try {
    db.exec(CREATE_MIGRATIONS_TABLE)
    insertMigrationRow(db, {
      version: 1,
      name: FIRST_MIGRATION_NAME,
      checksum: DIVERGED_WRONG_CHECKSUM,
      kind: "additive",
      downSql: null,
      dataDown: "full",
      appliedAt: Date.now()
    })
    db.exec(`PRAGMA user_version = 1;`)
  } finally {
    db.close()
  }
}

// ---------------------------------------------------------------------------
// DB_DOWNGRADE_FAILED
// ---------------------------------------------------------------------------

const DOWNGRADE_FAILED_MIGRATION_NAME = "seed_synthetic_breaking_no_down"

/**
 * A single `_migrations` row one version past this binary's own count,
 * marked `breaking` with `down_sql = NULL`. `compat.rs`'s `down_run` refuses
 * immediately when a breaking migration ahead has no stored down
 * (`CENSUS-RULE: compat.downgrade-breaking-no-down`) — a real defensive
 * path, not a fabricated one: a `_migrations` row with a null `down_sql` for
 * a `breaking`-kind migration is exactly what a torn/tampered metadata row
 * looks like, which is the condition this branch exists to catch.
 *
 * Exactly one version ahead (`count + 1`) rather than a large sentinel:
 * unlike `BACKWARDS_MIGRATION`, `handle_ahead` requires EVERY version in
 * `(count, user_version]` to have a row or it refuses as
 * `BackwardsMigration` first — so this needs the real `count`
 * (`migrationCount()`), not a headroom constant.
 */
function seedDbDowngradeFailed(runtimePath: string): void {
  const { main } = dbPaths(runtimePath)
  const version = migrationCount() + 1
  const db = openWritable(main)
  try {
    db.exec(CREATE_MIGRATIONS_TABLE)
    insertMigrationRow(db, {
      version,
      name: DOWNGRADE_FAILED_MIGRATION_NAME,
      // Excluded from the divergence check (version > count), so its exact
      // value is unobserved by the runner — kept deterministic anyway so the
      // unit test has something concrete to assert.
      checksum: sha256Hex(`${DOWNGRADE_FAILED_MIGRATION_NAME}@${version}`),
      kind: "breaking",
      downSql: null,
      dataDown: "full",
      appliedAt: Date.now()
    })
    db.exec(`PRAGMA user_version = ${version};`)
  } finally {
    db.close()
  }
}

// ---------------------------------------------------------------------------
// DB_DOWNGRADED
// ---------------------------------------------------------------------------

/**
 * Reconstructs directly-executable DDL statements from a `type|name|
 * tbl_name|sql` dump — a line-for-line TypeScript port of `schema_dump.rs`'s
 * `executable_statements`, which is what `compat.rs`'s `install_baseline`
 * replays `crates/carbon_repos/baseline/baseline.sql` through on a fresh
 * install. Needed here (rather than executing `baseline.sql` as raw SQL)
 * because that file is not raw SQL — it is `dump_schema`'s own pipe-
 * delimited output format, committed as the fresh-install fast path.
 *
 * Table statements are emitted before index/trigger/view statements
 * regardless of the dump's own (alphabetical, by type then name) line order,
 * matching the Rust function exactly; an index or trigger's owning table
 * must already exist. `sqlite_`-prefixed names (the engine's own
 * `sqlite_sequence`, and auto-index lines whose `sql` field is empty) are
 * skipped for the same reason the Rust function skips them: re-issuing their
 * `CREATE` is a reserved-name error there too.
 */
export function executableStatementsFromDump(dump: string): string[] {
  const tables: string[] = []
  const indexes: string[] = []
  const triggers: string[] = []
  const views: string[] = []
  for (const row of parseDump(dump)) {
    if (row.sql === "" || row.name.startsWith("sqlite_")) continue
    const stmt = `${row.sql};`
    if (row.type === "table") tables.push(stmt)
    else if (row.type === "index") indexes.push(stmt)
    else if (row.type === "trigger") triggers.push(stmt)
    else if (row.type === "view") views.push(stmt)
  }
  return [...tables, ...indexes, ...triggers, ...views]
}

interface DumpRow {
  type: string
  name: string
  tblName: string
  sql: string
}

/**
 * Splits `line` on `sep` like Rust's `str::splitn(n, sep)`: at most `n`
 * pieces, with any separator occurrences beyond the `(n-1)`th left intact in
 * the final piece. JavaScript's `String.prototype.split(sep, limit)` does
 * NOT have this behavior — it truncates the result to `limit` elements and
 * silently drops everything after, which would corrupt any `sql` field that
 * itself contains a literal `|`. `dump_schema`'s own line format
 * (`type|name|tbl_name|sql`) is exactly this shape (`splitn(4, '|')` in
 * `schema_dump.rs`), so this must match it precisely.
 */
function splitN(line: string, sep: string, n: number): string[] {
  const parts: string[] = []
  let rest = line
  for (let i = 0; i < n - 1; i++) {
    const idx = rest.indexOf(sep)
    if (idx === -1) break
    parts.push(rest.slice(0, idx))
    rest = rest.slice(idx + sep.length)
  }
  parts.push(rest)
  return parts
}

function parseDump(dump: string): DumpRow[] {
  return dump
    .split("\n")
    .filter((line) => line !== "")
    .map((line) => {
      const parts = splitN(line, "|", 4)
      return {
        type: parts[0] ?? "",
        name: parts[1] ?? "",
        tblName: parts[2] ?? "",
        sql: parts[3] ?? ""
      }
    })
}

/**
 * Every table name `baseline.sql` declares — this binary's own schema at its
 * own migration count. Exported for `dbSeed.test.ts`'s downgrade round-trip
 * assertion.
 *
 * Deliberately does NOT drop `sqlite_`-prefixed names here, unlike
 * `executableStatementsFromDump`'s skip of them: that skip exists only
 * because re-issuing `sqlite_sequence`'s own `CREATE TABLE` is a
 * reserved-name error on replay (the engine creates it automatically the
 * first time an `AUTOINCREMENT` table is created — several baseline tables
 * are). `schema_dump.rs`'s `dump_schema` itself excludes only `_migrations`,
 * `_prisma_migrations`, and `sqlite_stat*` (its own doc comment and the
 * `auto_indexes_are_still_part_of_the_dump` test) — `sqlite_sequence` is
 * real schema by that definition, and baseline.sql (being `dump_schema`'s
 * own committed output) already reflects that by carrying a
 * `table|sqlite_sequence|...` line. This function's job is to name what a
 * real `dump_schema` comparison considers "this binary's tables", so it must
 * agree.
 */
export function baselineTableNames(): string[] {
  const dump = fs.readFileSync(BASELINE_SQL_PATH, "utf8")
  return parseDump(dump)
    .filter((row) => row.type === "table")
    .map((row) => row.name)
}

const DOWNGRADE_MIGRATION_NAME = "seed_synthetic_breaking_reversible"
/** A table with no relationship to the real schema, added to simulate this
 *  synthetic migration's "up" having already run. */
export const SYNTHETIC_DOWNGRADE_TABLE = "__seed_synthetic_breaking_tail"
const SYNTHETIC_DOWNGRADE_UP_SQL = `CREATE TABLE "${SYNTHETIC_DOWNGRADE_TABLE}" ( "id" INTEGER PRIMARY KEY )`
/** Cleanly undoes `SYNTHETIC_DOWNGRADE_UP_SQL` with nothing left behind —
 *  exported so the test can execute it directly and check the round-trip. */
export const SYNTHETIC_DOWNGRADE_DOWN_SQL = `DROP TABLE "${SYNTHETIC_DOWNGRADE_TABLE}";`

/**
 * This binary's own schema (replayed from the committed `baseline.sql`, spec
 * §11 — CI-verified byte-identical to replaying the full migration chain)
 * plus one synthetic table, recorded as a `breaking`-kind migration one
 * version ahead of this binary's own count whose stored `down_sql` cleanly
 * drops that table and nothing else.
 *
 * This is the one state that requires the actual schema to be present before
 * the down-run: `compat.rs`'s `down_run` executes the stored `down_sql`
 * against the real on-disk database and then verifies the result via
 * `dump_schema` against this binary's own in-memory reference schema
 * (`reference_schema(count)`, built by replaying its real migrations) —
 * BYTE-FOR-BYTE, not merely "no error". Starting from anything other than
 * this binary's real schema (e.g. an empty or hand-picked subset of tables)
 * would make that comparison fail and produce `DB_DOWNGRADE_FAILED`
 * instead — the opposite of what this seed claims. Starting from the real,
 * CI-verified baseline and adding one independently-reversible table is the
 * one construction that can honestly reach `Downgraded`: the down-run drops
 * exactly what was added, leaving precisely the baseline schema, which by
 * the CI invariant is `reference_schema(count)`.
 */
function seedDbDowngraded(runtimePath: string): void {
  const { main } = dbPaths(runtimePath)
  const version = migrationCount() + 1
  const db = openWritable(main)
  try {
    const dump = fs.readFileSync(BASELINE_SQL_PATH, "utf8")
    const statements = executableStatementsFromDump(dump)
    db.exec(statements.join("\n"))
    db.exec(`${SYNTHETIC_DOWNGRADE_UP_SQL};`)

    db.exec(CREATE_MIGRATIONS_TABLE)
    insertMigrationRow(db, {
      version,
      name: DOWNGRADE_MIGRATION_NAME,
      checksum: sha256Hex(SYNTHETIC_DOWNGRADE_UP_SQL),
      kind: "breaking",
      downSql: SYNTHETIC_DOWNGRADE_DOWN_SQL,
      dataDown: "full",
      appliedAt: Date.now()
    })
    db.exec(`PRAGMA user_version = ${version};`)
  } finally {
    db.close()
  }
}

// ---------------------------------------------------------------------------
// DB_MIGRATION_FAILED
// ---------------------------------------------------------------------------

/**
 * A database with a pre-existing `AppConfiguration` table (incompatible with
 * what migration 1 creates) and `user_version` left at its default of 0.
 * Migration 1 (`20240120134904_init`) unconditionally
 * `CREATE TABLE "AppConfiguration" (...)`s with no `IF NOT EXISTS`, so
 * `apply_pending`'s forward loop fails its very first statement with a
 * genuine SQLite "table already exists" error — a normal `SQLITE_ERROR`, not
 * `DatabaseCorrupt`/`NotADatabase`, so `db_bootstrap.rs`'s `is_corruption`
 * returns false and `classify_open` funnels it to the generic
 * `DB_MIGRATION_FAILED` rather than `DB_CORRUPT`. This is the honest,
 * distinct-from-corruption failure mode: a schema that does not match what
 * the forward migration chain expects at its recorded version, exactly what
 * partial tampering or an interrupted migration could leave behind.
 */
function seedDbMigrationFailed(runtimePath: string): void {
  const { main } = dbPaths(runtimePath)
  const db = openWritable(main)
  try {
    db.exec(`CREATE TABLE "AppConfiguration" ( "id" INTEGER PRIMARY KEY );`)
    // user_version defaults to 0, which routes `MigrationSet::open` into the
    // forward-apply path (`backfill_applied` + `apply_pending(conn, 0,
    // count)`) rather than the ahead/downgrade branch.
  } finally {
    db.close()
  }
}

/**
 * Plants `<runtimePath>/gdl_conf.db` (and clears its WAL sidecars and any
 * stray pre-downgrade snapshot) in `state`, ready for a real app launch
 * against `runtimePath` to discover on its very first open and emit the
 * corresponding `_STATUS_:<EVENT>` line.
 */
export async function seedDatabase(
  runtimePath: string,
  state: SeedState
): Promise<void> {
  resetRuntimeDb(runtimePath)
  switch (state) {
    case "DB_CORRUPT":
      return seedDbCorrupt(runtimePath)
    case "BACKWARDS_MIGRATION":
      return seedBackwardsMigration(runtimePath)
    case "DB_DIVERGED":
      return seedDbDiverged(runtimePath)
    case "DB_DOWNGRADE_FAILED":
      return seedDbDowngradeFailed(runtimePath)
    case "DB_DOWNGRADED":
      return seedDbDowngraded(runtimePath)
    case "DB_MIGRATION_FAILED":
      return seedDbMigrationFailed(runtimePath)
  }
}
