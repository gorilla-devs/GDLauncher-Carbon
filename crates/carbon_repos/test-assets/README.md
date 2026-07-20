# Golden PCR fixture database

`golden_pcr.db` is a committed SQLite database written **through the Prisma Client
Rust (PCR) layer** — the same quaint-based encoding that shipped launchers wrote to
disk. It is the compatibility regression net: the new rusqlite reader must decode
bytes PCR produced, not bytes it produced itself.

## Contents (synthetic, deterministic — no real tokens)

- `Java` row `id = "golden-java-id"`, `major = 17`, `path = "/golden/java"`.
- `Account` row `uuid = "golden-uuid"`, `username = "GoldenUser"`, with
  `tokenExpires` and `lastUsed` set to `1784557692104` epoch milliseconds.

The load-bearing invariant is the on-disk datetime encoding: PCR/quaint stores
`DateTime` as an INTEGER of `timestamp_millis()`. `tests/golden.rs` asserts the
raw storage class is `integer`, the raw value equals the known millis, and that
`DbDateTime` decodes it back to the same instant.

## Regenerating

Only possible while the PCR layer still exists. From the repo root:

```bash
cargo test -p carbon_repos --test golden -- --ignored regenerate
```

This deletes and rewrites `golden_pcr.db`, checkpoints the WAL back into the main
file, then commit the updated asset. The verification test runs by default:

```bash
cargo test -p carbon_repos --test golden
```
