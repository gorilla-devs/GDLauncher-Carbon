# Fresh-install schema baseline

`baseline.sql` is the one committed schema artifact with two consumers (spec
§11):

1. **`tests/schema_snapshot.rs`** byte-compares it against the normalized
   schema dump ([`carbon_repos::schema_dump::dump_schema`]) produced by
   replaying the full migration chain — the regression net for the bundled
   SQLite version.
2. **The fresh-install runner** (`MigrationSet::open` in `src/compat.rs`)
   reconstructs a dependency-safe, directly executable DDL sequence from it
   (`schema_dump::executable_statements`) and runs it in one transaction
   instead of replaying every historical migration, when it opens a database
   with an empty `sqlite_master`.

Both consumers read the exact same file, so there is nothing to keep in sync
by hand: `tests/baseline.rs` is the CI-fatal equivalence test asserting the
baseline path and the chain path produce byte-identical schemas, `_migrations`
metadata, and (empty) data.

## Format

One line per `sqlite_master` row: `type|name|tbl_name|sql`, whitespace-
normalized, ordered by `(type, name)` — identical to `dump_schema`'s output.
SQLite's implicit auto-indexes (created by a table's own `PRIMARY KEY` /
`UNIQUE` constraint) appear with an empty `sql` field and are not re-executed.

## Regenerating

Regenerated automatically by the `new_migration` tool every time it
successfully generates or verifies a new migration's `down.sql` — the
committed file always reflects the full chain including the newest migration.
To regenerate by hand from the current chain:

```bash
cargo test -p carbon_repos --test schema_snapshot -- --ignored generate_snapshot
```
