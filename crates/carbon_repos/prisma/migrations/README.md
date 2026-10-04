# Migrations directory

This directory holds every migration this binary has ever shipped, one
subdirectory per migration in chronological (timestamp-prefixed) order. The
`prisma/migrations` path is kept for path stability only — there is no Prisma
involved; each `migration.sql` is plain SQL, and `../../src/lib.rs`'s
`get_migrations()` is the single source of truth binding a directory to its
`MigrationDef` (up SQL, down SQL, kind, lossiness).

## Shipped `migration.sql` files are immutable

Once a migration has shipped in a release, its `migration.sql` must never be
edited again — not even a reformat, an added or corrected comment, or a
trailing-newline change picked up by the repo's `.editorconfig`.

`compat.rs`'s `MigrationSet::checksum` hashes a migration's raw `up_sql` bytes,
and every existing install recorded that exact checksum in its own
`_migrations` table the moment the migration first applied. The schema tests
(`schema_snapshot`, `baseline`, `cross_version`, …) only compare the resulting
*schema*, so a byte-level edit that leaves the schema unchanged passes CI
cleanly — but it changes the checksum. Every existing install then computes a
value that disagrees with the one it already recorded, and
`compat::MigrationSet::open` refuses the database as `Diverged`. The only
recovery rung the failure screen offers from there is "Reset Database" — a
full wipe of accounts, settings, and instance metadata.

`tests/migration_checksums_frozen.rs` freezes the exact `(name, sha256)` for
every shipped migration as string literals precisely to catch this kind of
edit locally, before it ships. If that test fails on a migration you did not
believe you were touching, you (or a formatter/linter) edited a shipped
`migration.sql` — revert the change; write a new migration instead.

## Down scripts must succeed on users' data

An older build that opens a database a newer build migrated runs the stored
`down.sql` of every breaking migration it does not know. That text is copied
into the database's `_migrations` table when the migration first applies and
is never refreshed, so once a release ships a migration its `down.sql` is as
frozen as its `migration.sql` (the checksum covers only `migration.sql`, so
nothing catches an edit; it just never reaches existing databases).

A down that fails rolls back and leaves the user on the fatal "database
downgrade failed" screen, so write it to succeed on any data the newer build
can create. Rows of cache tables (`ModFileCache`, `ServerModFileCache`, the
CurseForge and Modrinth caches) that the older schema cannot hold are deleted,
never refused with `RAISE(ABORT)`: the older build rebuilds them by rescanning.
Foreign keys are off while a down runs, so delete dependent rows explicitly.

## Adding a migration

Do not create a directory here by hand. Use the generator:

```bash
cargo run -p carbon_repos --bin new_migration -- <name>
```

Run once to scaffold `prisma/migrations/<timestamp>_<name>/migration.sql`,
write the forward SQL there, then rerun the same command. On the second run
the tool derives `down.sql`, derives `kind` and lossiness, verifies the down
round-trips the prior schema (including a seeded boundary-value data
round-trip), regenerates `baseline/baseline.sql`, inserts the `MigrationDef`
into `get_migrations()` in `src/lib.rs`, and appends the new
`(name, checksum)` tuple to `tests/migration_checksums_frozen.rs`'s `FROZEN`
list. The last two insertions are best-effort — each prints the exact text to
paste by hand if its anchor comment is missing or ambiguous — but
`migration_checksums_frozen.rs`'s own migration-count check fails loudly in CI
if a new migration is left out of `FROZEN` regardless, so nothing ships
silently uncovered.

Renames and DML on a pre-existing table cannot be auto-derived: the tool stops
with a non-zero exit until you hand-write the reverse (renames) or pass
`--dml-reviewed` with a hand-verified `down.sql` (DML).
