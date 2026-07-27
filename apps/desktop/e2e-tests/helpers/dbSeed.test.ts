import { DatabaseSync } from "node:sqlite"
import fs from "node:fs/promises"
import fsSync from "node:fs"
import os from "node:os"
import path from "node:path"
import { fileURLToPath } from "node:url"
import { afterEach, beforeEach, describe, expect, it } from "vitest"
import {
  baselineTableNames,
  firstMigrationRealChecksum,
  migrationCount,
  seedDatabase,
  SYNTHETIC_DOWNGRADE_DOWN_SQL,
  SYNTHETIC_DOWNGRADE_TABLE,
  type SeedState
} from "./dbSeed.js"
import { withConfigDb } from "./versionCache.js"

const __dirname = path.dirname(fileURLToPath(import.meta.url))
// helpers -> e2e-tests -> desktop -> apps -> repo root, same resolution
// dbSeed.ts itself uses.
const REPO_ROOT = path.resolve(__dirname, "..", "..", "..", "..")
const MIGRATIONS_DIR = path.join(
  REPO_ROOT,
  "crates/carbon_repos/prisma/migrations"
)
const LIB_RS_PATH = path.join(REPO_ROOT, "crates/carbon_repos/src/lib.rs")
const FIRST_MIGRATION_SQL_PATH = path.join(
  REPO_ROOT,
  "crates/carbon_repos/prisma/migrations/20240120134904_init/migration.sql"
)

/**
 * Independently re-derives the migration name set straight from
 * `get_migrations()` in `crates/carbon_repos/src/lib.rs` — the single source
 * of truth (root `CLAUDE.md`) — rather than trusting `dbSeed.ts`'s own
 * `migrationCount()`, which only counts `prisma/migrations/` directories.
 *
 * This exists because `migrationCount()` is what `dbSeed.ts`'s seed
 * functions themselves use to compute a seeded `_migrations.version` (e.g.
 * `seedDbDowngradeFailed`'s `migrationCount() + 1`). A test whose own
 * "expected" version is ALSO computed via `migrationCount()` cannot catch
 * `migrationCount()` disagreeing with the real Rust classifier — both sides
 * of the assertion would drift together and stay green. Parsing the actual
 * source `get_migrations()` binds against gives an expectation that is wrong
 * in exactly the way the real classifier would be wrong, so a real
 * directory/source mismatch fails this loudly instead of passing
 * self-consistently (see task-3-review.md point 4).
 *
 * Handles both entry forms `get_migrations()` can contain: the
 * `historical_migration!("<name>")` macro (every entry today, pre-floor) and
 * a literal `MigrationDef { name: "<name>", ... }` struct (what
 * `cargo run -p carbon_repos --bin new_migration` prints to paste in for
 * migrations authored after the floor, per root `CLAUDE.md`).
 */
function migrationNamesFromRustSource(): string[] {
  const source = fsSync.readFileSync(LIB_RS_PATH, "utf8")

  const fnStart = source.indexOf("pub fn get_migrations()")
  if (fnStart === -1) {
    throw new Error(
      "migrationNamesFromRustSource: could not find `pub fn get_migrations()` in lib.rs — has it moved or been renamed?"
    )
  }
  const bodyStart = source.indexOf("{", fnStart)

  // Brace-counting rather than a regex up to the next `}`: the function body
  // contains nested `{}` (the `MigrationSet` struct literal, any
  // `MigrationDef { ... }` entries), so the first `}` is not the real end.
  let depth = 0
  let bodyEnd = -1
  for (let i = bodyStart; i < source.length; i++) {
    if (source[i] === "{") depth++
    else if (source[i] === "}") {
      depth--
      if (depth === 0) {
        bodyEnd = i
        break
      }
    }
  }
  if (bodyEnd === -1) {
    throw new Error(
      "migrationNamesFromRustSource: unbalanced braces while scanning get_migrations()'s body"
    )
  }

  const body = source.slice(bodyStart, bodyEnd)
  const names = new Set<string>()
  for (const m of body.matchAll(/historical_migration!\(\s*"([^"]+)"\s*\)/g)) {
    names.add(m[1])
  }
  for (const m of body.matchAll(/name:\s*"([^"]+)"/g)) {
    names.add(m[1])
  }

  if (names.size === 0) {
    throw new Error(
      "migrationNamesFromRustSource: parsed zero migration names out of get_migrations() — the parser is broken, not the source"
    )
  }

  return [...names]
}

let runtimePath: string

beforeEach(async () => {
  runtimePath = await fs.mkdtemp(path.join(os.tmpdir(), "gdl-e2e-dbseed-"))
})

afterEach(async () => {
  await fs.rm(runtimePath, { recursive: true, force: true })
})

function dbFile(): string {
  return path.join(runtimePath, "gdl_conf.db")
}

function walFile(): string {
  return `${dbFile()}-wal`
}

function shmFile(): string {
  return `${dbFile()}-shm`
}

/** Every table/index/trigger/view name currently in `sqlite_master`. */
function sqliteMasterNames(db: DatabaseSync): string[] {
  return (
    db.prepare(`SELECT name FROM sqlite_master`).all() as { name: string }[]
  ).map((r) => r.name)
}

/** Just the `type = 'table'` names in `sqlite_master` — excludes indexes,
 *  triggers, and views, which share the same namespace and would otherwise
 *  pollute a table-set comparison. */
function sqliteMasterTableNames(db: DatabaseSync): string[] {
  return (
    db.prepare(`SELECT name FROM sqlite_master WHERE type = 'table'`).all() as {
      name: string
    }[]
  ).map((r) => r.name)
}

function userVersion(db: DatabaseSync): number {
  return (db.prepare(`PRAGMA user_version`).get() as { user_version: number })
    .user_version
}

describe("migrationCount() independent cross-check", () => {
  it("the migration directory listing, get_migrations()'s Rust source, and migrationCount() all name the same migrations", () => {
    const fromSource = migrationNamesFromRustSource()
    const fromDirs = fsSync
      .readdirSync(MIGRATIONS_DIR, { withFileTypes: true })
      .filter((entry) => entry.isDirectory())
      .map((entry) => entry.name)

    // Set equality, not just count: catches a same-count-but-wrong-membership
    // drift (e.g. a renamed directory) that a bare length comparison would
    // miss.
    expect(new Set(fromDirs)).toEqual(new Set(fromSource))

    // `migrationCount()` itself must agree with the independently-derived
    // count — this is the actual property the seed functions below depend
    // on being true, and the one a same-function comparison could never
    // catch drifting.
    expect(migrationCount()).toBe(fromSource.length)
  })
})

describe("seedDatabase", () => {
  it("clears a stale WAL sidecar from a previous run before seeding", async () => {
    // Simulate leftovers from a previous app run/seed: a main file plus WAL
    // sidecars containing uncheckpointed data. A seed that merely overwrote
    // the main file and left these in place would risk the next real app
    // open resurrecting stale pages from the sidecar instead of seeing the
    // planted state — exactly the failure mode the brief warns about.
    await fs.writeFile(dbFile(), "old main file")
    await fs.writeFile(walFile(), "stale wal contents")
    await fs.writeFile(shmFile(), "stale shm contents")

    await seedDatabase(runtimePath, "DB_CORRUPT")

    expect(fsSync.existsSync(walFile())).toBe(false)
    expect(fsSync.existsSync(shmFile())).toBe(false)
  })

  describe("DB_CORRUPT", () => {
    it("writes a file that is not a valid SQLite database", async () => {
      await seedDatabase(runtimePath, "DB_CORRUPT")

      const bytes = await fs.readFile(dbFile())
      // The real SQLite file-format magic header is exactly these 16 bytes
      // ("SQLite format 3\0"); a corrupt/non-database file must not start
      // with it, or `db_bootstrap.rs`'s runner would not see NotADatabase at
      // all and this would silently stop testing DB_CORRUPT.
      const sqliteMagic = Buffer.from("SQLite format 3\0", "latin1")
      expect(bytes.subarray(0, 16).equals(sqliteMagic)).toBe(false)

      // A real reader must fail against it — not just "look wrong" by header
      // inspection. SQLite parses the header lazily (matching
      // `db_bootstrap.rs`'s own `corrupt_db_file_reports_corrupt_status`
      // test comment: "opened lazily; the first header read ... surfaces
      // NOTADB/CORRUPT"), so `new DatabaseSync` itself does not throw — the
      // first real query against it does.
      const db = new DatabaseSync(dbFile(), { readOnly: true })
      try {
        expect(() => db.prepare(`SELECT * FROM sqlite_master`).get()).toThrow()
      } finally {
        db.close()
      }
    })

    it("leaves no WAL sidecars behind", async () => {
      await seedDatabase(runtimePath, "DB_CORRUPT")
      expect(fsSync.existsSync(walFile())).toBe(false)
      expect(fsSync.existsSync(shmFile())).toBe(false)
    })
  })

  describe("BACKWARDS_MIGRATION", () => {
    it("has a non-empty schema, user_version far ahead of any real count, and no _migrations table", async () => {
      await seedDatabase(runtimePath, "BACKWARDS_MIGRATION")

      withConfigDb(runtimePath, (db) => {
        const names = sqliteMasterNames(db)
        // sqlite_master must be non-empty (else `MigrationSet::open` takes
        // the fresh-install baseline path instead of this branch), but must
        // NOT already contain `_migrations` — the real binary's own
        // `ensure_migrations_table` (`CREATE TABLE IF NOT EXISTS`) is what
        // must create it empty, which is the exact "metadata missing"
        // condition `BACKWARDS_MIGRATION` needs.
        expect(names.length).toBeGreaterThan(0)
        expect(names).not.toContain("_migrations")

        expect(userVersion(db)).toBeGreaterThan(migrationCount())
      })
    })
  })

  describe("DB_DIVERGED", () => {
    it("records a version-1 checksum that does not match this binary's real one", async () => {
      await seedDatabase(runtimePath, "DB_DIVERGED")

      withConfigDb(runtimePath, (db) => {
        expect(userVersion(db)).toBe(1)

        const row = db
          .prepare(
            `SELECT version, checksum FROM _migrations WHERE version = 1`
          )
          .get() as { version: number; checksum: string } | undefined

        expect(row).toBeDefined()
        expect(row!.checksum).not.toBe(firstMigrationRealChecksum())
        // A real sha256 hex digest is exactly 64 lowercase hex chars — assert
        // the stored value is shaped like a checksum, not merely "some
        // string", even though it's deliberately wrong.
        expect(row!.checksum).toMatch(/^[0-9a-f]{64}$/)
      })
    })
  })

  describe("DB_DOWNGRADE_FAILED", () => {
    it("records one breaking migration past this binary's count with no stored down", async () => {
      await seedDatabase(runtimePath, "DB_DOWNGRADE_FAILED")

      // Independently-derived, not `migrationCount()`: this is what actually
      // catches `seedDbDowngradeFailed`'s own internal `migrationCount()`
      // call disagreeing with the real Rust classifier (see
      // `migrationNamesFromRustSource`'s doc comment).
      const count = migrationNamesFromRustSource().length
      withConfigDb(runtimePath, (db) => {
        expect(userVersion(db)).toBe(count + 1)

        const row = db
          .prepare(
            `SELECT version, kind, down_sql FROM _migrations WHERE version = ?`
          )
          .get(count + 1) as
          | { version: number; kind: string; down_sql: string | null }
          | undefined

        expect(row).toBeDefined()
        expect(row!.kind).toBe("breaking")
        expect(row!.down_sql).toBeNull()

        // Nothing recorded at or below this binary's own count: divergence
        // must find nothing to compare, so this state is reached only via
        // the "breaking migration ahead with no down" branch, never
        // DB_DIVERGED.
        const belowCount = db
          .prepare(`SELECT COUNT(*) AS n FROM _migrations WHERE version <= ?`)
          .get(count) as { n: number }
        expect(belowCount.n).toBe(0)
      })
    })
  })

  describe("DB_DOWNGRADED", () => {
    it("carries this binary's real baseline schema plus one genuinely reversible breaking migration", async () => {
      await seedDatabase(runtimePath, "DB_DOWNGRADED")

      // Independently-derived — see the `DB_DOWNGRADE_FAILED` test above and
      // `migrationNamesFromRustSource`'s doc comment.
      const count = migrationNamesFromRustSource().length
      const realTables = new Set(baselineTableNames())

      withConfigDb(runtimePath, (db) => {
        expect(userVersion(db)).toBe(count + 1)

        const row = db
          .prepare(`SELECT kind, down_sql FROM _migrations WHERE version = ?`)
          .get(count + 1) as { kind: string; down_sql: string } | undefined

        expect(row).toBeDefined()
        expect(row!.kind).toBe("breaking")
        expect(row!.down_sql).toBe(SYNTHETIC_DOWNGRADE_DOWN_SQL)

        const names = new Set(sqliteMasterTableNames(db))
        // Every real baseline table is present...
        for (const table of realTables) {
          expect(names.has(table)).toBe(true)
        }
        // ...plus the synthetic addition simulating the migration's "up"
        // having already been applied, which is what a down-run needs to
        // find and reverse.
        expect(names.has(SYNTHETIC_DOWNGRADE_TABLE)).toBe(true)
      })

      // Round-trip the exact down_sql compat.rs's down_run would execute,
      // directly against the seeded file, and confirm it leaves precisely
      // this binary's own baseline table set behind — the property
      // `down_run`'s schema-dump comparison (`dump_schema(&tx) == reference`)
      // depends on, exercised here without reimplementing the Rust
      // normalizer.
      const db = new DatabaseSync(dbFile())
      try {
        db.exec(SYNTHETIC_DOWNGRADE_DOWN_SQL)
        const namesAfter = new Set(sqliteMasterTableNames(db))
        namesAfter.delete("_migrations")
        expect(namesAfter).toEqual(realTables)
      } finally {
        db.close()
      }
    })
  })

  describe("DB_MIGRATION_FAILED", () => {
    it("leaves a pre-existing AppConfiguration table colliding with migration 1, at user_version 0", async () => {
      await seedDatabase(runtimePath, "DB_MIGRATION_FAILED")

      withConfigDb(runtimePath, (db) => {
        expect(userVersion(db)).toBe(0)
        expect(sqliteMasterNames(db)).toContain("AppConfiguration")
      })

      // The real proof: replaying migration 1's actual up_sql (the exact
      // text `include_str!`'d into the running binary) against this seeded
      // file must fail with a genuine "already exists" collision — the
      // concrete mechanism `apply_pending`'s forward loop hits, not merely
      // an assumption that it would.
      const migrationSql = await fs.readFile(FIRST_MIGRATION_SQL_PATH, "utf8")
      const db = new DatabaseSync(dbFile())
      try {
        let thrown: unknown
        try {
          db.exec(migrationSql)
        } catch (e) {
          thrown = e
        }
        expect(thrown).toBeDefined()
        expect(String(thrown)).toMatch(/already exists/i)
      } finally {
        db.close()
      }
    })
  })

  it("clears a stale pre-downgrade snapshot before seeding", async () => {
    const snapshotPath = path.join(runtimePath, "gdl_conf.pre-downgrade.db")
    await fs.writeFile(snapshotPath, "stale snapshot from a previous session")

    await seedDatabase(runtimePath, "DB_CORRUPT")

    expect(fsSync.existsSync(snapshotPath)).toBe(false)
  })

  it("every SeedState is handled without throwing", async () => {
    const states: SeedState[] = [
      "DB_CORRUPT",
      "BACKWARDS_MIGRATION",
      "DB_DIVERGED",
      "DB_DOWNGRADE_FAILED",
      "DB_DOWNGRADED",
      "DB_MIGRATION_FAILED"
    ]
    for (const state of states) {
      await expect(seedDatabase(runtimePath, state)).resolves.toBeUndefined()
    }
  })
})
