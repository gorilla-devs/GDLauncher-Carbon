//! `new_migration` — the migration authoring tool.
//!
//! Two phases, keyed on whether the migration directory's `migration.sql`
//! already holds a written `up`:
//!
//! 1. **Scaffold** — `new_migration <name>` with no matching directory creates
//!    `prisma/migrations/<utc-timestamp>_<name>/migration.sql` from an empty
//!    template. The developer writes the forward SQL there.
//! 2. **Generate** — rerun `new_migration <name>` once the `up` is written and
//!    the tool derives `down.sql` by schema diff, verifies it
//!    round-trips the prior schema, and prints the `MigrationDef` entry to paste
//!    into `get_migrations()`.
//!
//! Two touchpoints the diff cannot resolve stop generation with a non-zero exit
//! until the developer resolves them: a **rename** (hand-write the reverse rename
//! in `down.sql`) and **DML on a pre-existing table** (pass `--dml-reviewed` and
//! supply an inverse `down.sql`). A hand-written `down.sql` is never overwritten
//! — it is verified to round-trip instead.
//!
//! The runtime never runs this tool; it executes the reviewed, committed,
//! CI-verified scripts.

use carbon_repos::compat::{MigrationKind, sha256_hex};
use carbon_repos::downgen::{
    GenError, analyze_up, full_schema_dump, generate_down, insert_migration_entry,
    verify_round_trip,
};
use carbon_repos::manifest::{DataDown, derive_kind, seeded_lost_fields};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const MIGRATIONS_SUBDIR: &str = "prisma/migrations";
const BASELINE_PATH: &str = "baseline/baseline.sql";
const LIB_RS_PATH: &str = "src/lib.rs";
const CHECKSUMS_TEST_PATH: &str = "tests/migration_checksums_frozen.rs";

/// The exact anchor comment `tests/migration_checksums_frozen.rs` carries
/// inside its `FROZEN` array. `new_migration` inserts each new migration's
/// `(name, checksum)` tuple directly above this line, mirroring how
/// [`MIGRATION_LIST_ANCHOR`](carbon_repos::downgen::MIGRATION_LIST_ANCHOR)
/// works for `lib.rs`.
const CHECKSUM_LIST_ANCHOR: &str = "// new-migration:anchor — new_migration appends the new (name, checksum) tuple directly above this line";

const TEMPLATE: &str = "-- Write the forward (up) SQL for this migration here, then rerun\n\
-- `cargo run -p carbon_repos --bin new_migration -- <name>` to generate down.sql.\n";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut name: Option<String> = None;
    let mut dml_reviewed = false;
    for arg in &args {
        match arg.as_str() {
            "--dml-reviewed" => dml_reviewed = true,
            other if other.starts_with("--") => {
                eprintln!("unknown flag: {other}");
                return ExitCode::FAILURE;
            }
            other if name.is_none() => name = Some(other.to_string()),
            other => {
                eprintln!("unexpected extra argument: {other}");
                return ExitCode::FAILURE;
            }
        }
    }
    let Some(name) = name else {
        eprintln!(
            "usage: cargo run -p carbon_repos --bin new_migration -- <name> [--dml-reviewed]"
        );
        return ExitCode::FAILURE;
    };

    let migrations_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(MIGRATIONS_SUBDIR);
    match run(&migrations_root, &name, dml_reviewed) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Drives the scaffold-or-generate flow; returns the process exit code.
fn run(migrations_root: &Path, name: &str, dml_reviewed: bool) -> std::io::Result<ExitCode> {
    let dirs = ordered_migration_dirs(migrations_root)?;

    let existing = find_migration_dir(&dirs, name);

    let Some(dir) = existing else {
        return scaffold(migrations_root, name);
    };

    if let Err(msg) = require_newest(&dirs, &dir) {
        eprintln!("{msg}");
        return Ok(ExitCode::FAILURE);
    }

    let up_path = dir.join("migration.sql");
    let up = std::fs::read_to_string(&up_path).unwrap_or_default();
    if is_effectively_empty(&up) {
        eprintln!(
            "migration.sql in {} has no SQL yet.\n\
             Write the forward (up) SQL, then rerun this tool.",
            dir.display()
        );
        return Ok(ExitCode::FAILURE);
    }

    // Predecessors: every migration whose directory sorts before this one.
    let this_dir_name = dir_name(&dir).unwrap_or_default().to_string();
    let prev_owned: Vec<String> = dirs
        .iter()
        .filter(|d| {
            dir_name(d)
                .map(|n| n < this_dir_name.as_str())
                .unwrap_or(false)
        })
        .map(|d| std::fs::read_to_string(d.join("migration.sql")).unwrap_or_default())
        .collect();
    let prev: Vec<&str> = prev_owned.iter().map(|s| s.as_str()).collect();

    generate_or_verify(&dir, &this_dir_name, &prev, &up, dml_reviewed)
}

/// Creates a fresh timestamped scaffold directory with an empty `migration.sql`.
fn scaffold(migrations_root: &Path, name: &str) -> std::io::Result<ExitCode> {
    let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S").to_string();
    let dir_name = format!("{timestamp}_{name}");
    let dir = migrations_root.join(&dir_name);
    std::fs::create_dir_all(&dir)?;
    let up_path = dir.join("migration.sql");
    std::fs::write(&up_path, TEMPLATE)?;
    println!("Scaffolded {}", dir.display());
    println!(
        "Write the forward (up) SQL in {}, then rerun this tool to generate down.sql.",
        up_path.display()
    );
    Ok(ExitCode::SUCCESS)
}

/// With a written `up`: verify a hand-written `down.sql` if present, otherwise
/// generate one (unless a rename/DML touchpoint requires human action first).
fn generate_or_verify(
    dir: &Path,
    dir_name: &str,
    prev: &[&str],
    up: &str,
    dml_reviewed: bool,
) -> std::io::Result<ExitCode> {
    let analysis = match analyze_up(prev, up) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("could not analyse the up SQL: {e}");
            return Ok(ExitCode::FAILURE);
        }
    };

    // DML on a pre-existing table always requires the explicit review flag.
    if !analysis.dml_on_existing.is_empty() && !dml_reviewed {
        eprintln!(
            "This migration performs DML on pre-existing table(s): {}.\n\
             Data transforms are not auto-invertible. Write an inverse down.sql that restores\n\
             the prior representation, then rerun with --dml-reviewed.",
            analysis.dml_on_existing.join(", ")
        );
        return Ok(ExitCode::FAILURE);
    }

    let down_path = dir.join("down.sql");
    let down_exists = down_path.exists();

    if down_exists {
        let down = std::fs::read_to_string(&down_path)?;
        match verify_round_trip(prev, up, &down) {
            Ok(()) => {
                println!("Hand-written down.sql verified: it round-trips the prior schema.");
                regenerate_baseline(prev, up)?;
                let list_code = apply_list_entry(&default_lib_path(), dir_name, prev, up, &down)?;
                let checksum_code =
                    append_checksum_entry(&default_checksums_test_path(), dir_name, up)?;
                Ok(combine(list_code, checksum_code))
            }
            Err(e) => {
                eprintln!("Hand-written down.sql does NOT round-trip:\n{e}");
                Ok(ExitCode::FAILURE)
            }
        }
    } else if analysis.rename {
        eprintln!(
            "A rename (or rename-shaped drop+add) was detected in the up.\n\
             The schema diff cannot tell a rename from an unrelated drop+add, so hand-write the\n\
             reverse rename in {} and rerun; the tool will verify it round-trips.",
            down_path.display()
        );
        Ok(ExitCode::FAILURE)
    } else if !analysis.dml_on_existing.is_empty() {
        eprintln!(
            "DML on pre-existing table(s) needs a hand-written inverse down.sql.\n\
             Write it in {} and rerun with --dml-reviewed.",
            down_path.display()
        );
        Ok(ExitCode::FAILURE)
    } else {
        match generate_down(prev, up) {
            Ok(down) => {
                std::fs::write(&down_path, &down)?;
                println!("Generated {} (verified round-trip).", down_path.display());
                regenerate_baseline(prev, up)?;
                let list_code = apply_list_entry(&default_lib_path(), dir_name, prev, up, &down)?;
                let checksum_code =
                    append_checksum_entry(&default_checksums_test_path(), dir_name, up)?;
                Ok(combine(list_code, checksum_code))
            }
            Err(GenError::RoundTripFailed { expected, actual }) => {
                eprintln!(
                    "Down generation could not invert this migration automatically.\n\
                     Expected prior schema:\n{expected}\nGot after up+down:\n{actual}\n\
                     Hand-write {} and rerun.",
                    down_path.display()
                );
                Ok(ExitCode::FAILURE)
            }
            Err(e) => {
                eprintln!("Down generation failed: {e}");
                Ok(ExitCode::FAILURE)
            }
        }
    }
}

/// Regenerates the committed fresh-install baseline from `prev`
/// (every earlier migration's `up`) plus this migration's `up`. `new_migration`
/// only ever operates on the newest migration in the chain, so `prev + up` is
/// always the full chain's schema at this point — the exact content
/// `baseline/baseline.sql` must hold. Called after a down is generated or a
/// hand-written one verifies, so the committed baseline never lags behind a
/// successfully authored migration.
fn regenerate_baseline(prev: &[&str], up: &str) -> std::io::Result<()> {
    let baseline_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(BASELINE_PATH);
    let dump = full_schema_dump(prev, up).map_err(|e| {
        std::io::Error::other(format!(
            "failed to build schema for baseline regeneration: {e}"
        ))
    })?;
    std::fs::write(&baseline_path, dump)?;
    println!("Regenerated {}", baseline_path.display());
    Ok(())
}

/// Builds the `MigrationDef` list-entry text for this migration, with `kind`
/// and `data_down` **derived** from the up/down, not left as
/// placeholders: the same values the CI gate enforces are what the tool
/// inserts, so a correct edit passes by construction.
fn build_list_entry(dir_name: &str, prev: &[&str], up: &str, down: &str) -> String {
    let (kind_expr, kind_note) = match derive_kind(prev, up) {
        Ok(MigrationKind::Additive) => ("MigrationKind::Additive", String::new()),
        Ok(MigrationKind::Breaking) => ("MigrationKind::Breaking", String::new()),
        Err(e) => (
            "MigrationKind::Breaking",
            format!(" // could not derive kind ({e}); defaulting to Breaking — verify"),
        ),
    };

    let (data_down, data_note) = match seeded_lost_fields(prev, up, down) {
        Ok(lost) => {
            let decl = if lost.is_empty() {
                DataDown::Full
            } else {
                DataDown::Partial(lost)
            };
            (decl.to_declaration(), String::new())
        }
        Err(e) => (
            "full".to_string(),
            format!(" // could not seed round-trip ({e}); verify lossiness by hand"),
        ),
    };

    let mut out = String::new();
    out.push_str("        MigrationDef {\n");
    out.push_str(&format!("            name: \"{dir_name}\",\n"));
    out.push_str(&format!(
        "            up_sql: include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/prisma/migrations/{dir_name}/migration.sql\")),\n"
    ));
    out.push_str(&format!(
        "            down_sql: Some(include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/prisma/migrations/{dir_name}/down.sql\"))),\n"
    ));
    out.push_str(&format!("            kind: {kind_expr},{kind_note}\n"));
    out.push_str(&format!(
        "            data_down: \"{data_down}\",{data_note}\n"
    ));
    out.push_str("        },\n");
    out
}

/// This checkout's real `src/lib.rs` — the path [`apply_list_entry`] edits
/// during normal tool runs. Kept as its own function (rather than inlined at
/// the call sites) so tests can call `apply_list_entry` with a scratch path
/// instead, never touching the real source file.
fn default_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(LIB_RS_PATH)
}

/// Inserts this migration's `MigrationDef` entry into `get_migrations()` in
/// `lib_path` via [`insert_migration_entry`], replacing the manual copy-paste
/// step, and prints the inserted text as confirmation. A missing or
/// duplicated anchor is a hard failure (`FAILURE` exit) with the entry printed
/// so the developer can place it by hand — the migration's `up`/`down` are
/// already written and verified at this point, only the list edit is at risk.
fn apply_list_entry(
    lib_path: &Path,
    dir_name: &str,
    prev: &[&str],
    up: &str,
    down: &str,
) -> std::io::Result<ExitCode> {
    let entry = build_list_entry(dir_name, prev, up, down);
    let lib_src = std::fs::read_to_string(lib_path)?;
    match insert_migration_entry(&lib_src, &entry) {
        Ok(updated) => {
            if updated == lib_src {
                println!(
                    "\n{} already has an entry for {dir_name}; nothing to insert.\n",
                    lib_path.display()
                );
            } else {
                std::fs::write(lib_path, &updated)?;
                println!(
                    "\nInserted into get_migrations() in {}:\n",
                    lib_path.display()
                );
            }
            print!("{entry}");
            Ok(ExitCode::SUCCESS)
        }
        Err(e) => {
            eprintln!(
                "warning: could not auto-insert into {}: {e}\n\
                 Add this entry to get_migrations() by hand:\n",
                lib_path.display()
            );
            print!("{entry}");
            Ok(ExitCode::FAILURE)
        }
    }
}

/// This checkout's real `tests/migration_checksums_frozen.rs` — the path
/// [`append_checksum_entry`] edits during normal tool runs. Kept as its own
/// function, mirroring [`default_lib_path`], so tests can point the appender
/// at a scratch path instead.
fn default_checksums_test_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(CHECKSUMS_TEST_PATH)
}

/// Appends this migration's `(name, sha256-of-up)` tuple to the `FROZEN` list
/// in `checksums_path` (spec L1: the shipped-migration checksum fence),
/// mirroring how [`apply_list_entry`] inserts into `lib.rs`. Best-effort: a
/// missing or duplicated anchor is a warning with the tuple printed for a
/// manual paste, not a hard stop, but never silent — a migration missing from
/// `FROZEN` already fails `migration_checksums_frozen.rs`'s own count check
/// loudly, in CI, before it ships, so skipping this step is caught elsewhere
/// even when it isn't caught here.
///
/// Three outcomes, told apart before [`insert_before_anchor`] runs so each
/// gets its own message: no tuple named `dir_name` existed (fresh insert); one
/// existed and is byte-identical (true no-op); one existed but the checksum
/// differs — the up SQL changed since it was written — and is replaced in
/// place (see `run()`'s newest-in-chain guard for why that can only ever be
/// this migration's own, still-unshipped tuple).
fn append_checksum_entry(
    checksums_path: &Path,
    dir_name: &str,
    up: &str,
) -> std::io::Result<ExitCode> {
    let checksum = sha256_hex(up.as_bytes());
    let entry = format!("    (\n        \"{dir_name}\",\n        \"{checksum}\",\n    ),");
    let src = std::fs::read_to_string(checksums_path)?;
    let had_entry = src.contains(&tuple_needle(dir_name));
    match insert_before_anchor(&src, CHECKSUM_LIST_ANCHOR, &entry, dir_name) {
        Ok(updated) => {
            if updated == src {
                println!(
                    "{} already has a frozen checksum entry for {dir_name}; nothing to insert.",
                    checksums_path.display()
                );
            } else if had_entry {
                std::fs::write(checksums_path, &updated)?;
                println!("updated frozen checksum for {dir_name}");
            } else {
                std::fs::write(checksums_path, &updated)?;
                println!(
                    "Appended the frozen checksum entry for {dir_name} to {}.",
                    checksums_path.display()
                );
            }
            Ok(ExitCode::SUCCESS)
        }
        Err(e) => {
            eprintln!(
                "warning: could not auto-append the frozen checksum entry to {}: {e}\n\
                 Add this tuple to FROZEN by hand:\n{entry}",
                checksums_path.display()
            );
            Ok(ExitCode::FAILURE)
        }
    }
}

/// The exact quoted-name substring identifying a migration's tuple in the
/// `FROZEN` list — shared by [`append_checksum_entry`]'s own presence check
/// and [`insert_before_anchor`]'s, so both agree on what "already has an
/// entry for this migration" means.
fn tuple_needle(dir_name: &str) -> String {
    format!("\"{dir_name}\"")
}

/// Inserts `entry` directly above the line containing `anchor` in `src`,
/// returning the updated source.
///
/// Idempotent by content, not merely by name: if a tuple naming `dir_name` is
/// already present and byte-identical to `entry`, `src` is returned
/// unchanged — rerunning the tool for an unedited migration never duplicates
/// anything. If a tuple naming `dir_name` is present but differs (the up SQL
/// changed since that tuple was written, so its checksum no longer matches),
/// the stale tuple is replaced in place instead of left behind: leaving it
/// would pin `FROZEN` to a hash the migration no longer produces, and the
/// resulting `migration_checksums_frozen` failure would point the developer
/// at re-deriving a hash the tool could have just fixed. Only ever inserts
/// fresh, above `anchor`, when no tuple names `dir_name` at all.
///
/// INVARIANT: `run()` calls [`require_newest`] before any of this ever runs,
/// refusing every name except the chain's newest, locally-authored migration.
/// The replace branch here can therefore only ever rewrite *that* migration's
/// own tuple — it can never be reached for, and so can never silently
/// rewrite, a shipped mid-chain migration's frozen checksum.
///
/// Fails if `anchor` is missing or appears more than once (no single
/// unambiguous insertion point), or if a tuple names `dir_name` but is not in
/// the fixed 4-line shape this tool itself always writes (a hand-edited
/// `FROZEN` entry) — replacing an unrecognised shape would risk corrupting
/// unrelated text instead of just the stale tuple.
///
/// A smaller, file-agnostic sibling of [`insert_migration_entry`], which is
/// hard-wired to `lib.rs`'s own anchor constant.
fn insert_before_anchor(
    src: &str,
    anchor: &str,
    entry: &str,
    dir_name: &str,
) -> Result<String, String> {
    let anchor_count = src.matches(anchor).count();
    if anchor_count == 0 {
        return Err(format!("no `{anchor}` marker found"));
    }
    if anchor_count > 1 {
        return Err(format!(
            "more than one `{anchor}` marker found; the insertion point is ambiguous"
        ));
    }

    let needle = tuple_needle(dir_name);
    if let Some(needle_pos) = src.find(&needle) {
        if src.contains(entry) {
            return Ok(src.to_string());
        }
        let (start, end) = tuple_span(src, needle_pos).ok_or_else(|| {
            format!(
                "found `{needle}` but its tuple is not in the expected `(\"name\", \"checksum\"),` \
                 shape; fix it by hand"
            )
        })?;
        let mut out = String::with_capacity(src.len() + entry.len());
        out.push_str(&src[..start]);
        out.push_str(entry);
        out.push('\n');
        out.push_str(&src[end..]);
        return Ok(out);
    }

    let anchor_pos = src.find(anchor).expect("anchor_count == 1 checked above");
    let line_start = src[..anchor_pos].rfind('\n').map(|i| i + 1).unwrap_or(0);

    let mut out = String::with_capacity(src.len() + entry.len() + 1);
    out.push_str(&src[..line_start]);
    out.push_str(entry);
    out.push('\n');
    out.push_str(&src[line_start..]);
    Ok(out)
}

/// The byte span `[start, end)` of the tuple line-block containing byte offset
/// `needle_pos` — from the start of its opening `    (` line through the start
/// of the line following its closing `    ),` line (mirroring
/// [`insert_before_anchor`]'s own `line_start` semantics for the fresh-insert
/// path, so both branches build the replacement the same way). Assumes the
/// fixed 4-line shape [`append_checksum_entry`] always writes; returns `None`
/// if that exact shape is not found around `needle_pos`.
fn tuple_span(src: &str, needle_pos: usize) -> Option<(usize, usize)> {
    let open_nl = src[..needle_pos].rfind("\n    (\n")?;
    let start = open_nl + 1;

    let close_rel = src[needle_pos..].find("    ),\n")?;
    let end = needle_pos + close_rel + "    ),\n".len();

    Some((start, end))
}

/// `SUCCESS` only when both steps succeeded — the combined exit code for the
/// two independent "append an entry" steps ([`apply_list_entry`] and
/// [`append_checksum_entry`]) that run after a migration's down is verified.
fn combine(a: ExitCode, b: ExitCode) -> ExitCode {
    if a == ExitCode::SUCCESS && b == ExitCode::SUCCESS {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// The migration directories under `root`, sorted by name (timestamp prefix =
/// chronological). Ignores non-directories and files.
fn ordered_migration_dirs(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if !root.exists() {
        return Ok(dirs);
    }
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            dirs.push(entry.path());
        }
    }
    dirs.sort();
    Ok(dirs)
}

/// A path's final component as a string slice.
fn dir_name(path: &Path) -> Option<&str> {
    path.file_name().and_then(|n| n.to_str())
}

/// This migration's directory: the newest whose name portion — everything after
/// the leading `<timestamp>_` — equals `name` exactly.
///
/// Both halves matter. A suffix match would treat `servers` as naming
/// `20260223000000_add_servers`, and taking the first hit of an ascending list
/// would pick the oldest when a name is reused. Either way the tool would then
/// operate on a historical migration while every caller assumes it holds the
/// newest: `regenerate_baseline` would rewrite the committed baseline from a
/// truncated chain, silently dropping every later migration's objects.
fn find_migration_dir(dirs: &[PathBuf], name: &str) -> Option<PathBuf> {
    dirs.iter()
        .filter(|d| {
            dir_name(d)
                .and_then(|n| n.split_once('_'))
                .is_some_and(|(_timestamp, rest)| rest == name)
        })
        .next_back()
        .cloned()
}

/// Refuses `dir` unless it is `dirs`' newest entry (`dirs` is already sorted
/// chronologically by [`ordered_migration_dirs`]). `find_migration_dir` can
/// match an older directory whenever a *different*, newer migration has since
/// been added to the chain — the reused-name case it handles is only about
/// picking the newest among same-named directories, not about the directory
/// found being the chain's newest overall. Every caller downstream
/// (`regenerate_baseline`, `apply_list_entry`, `append_checksum_entry`)
/// assumes it is operating on the newest migration in the chain; running
/// against an older one would regenerate the committed baseline, the
/// `get_migrations()` list, and the frozen checksum from a truncated chain,
/// silently dropping every later migration's objects while reporting success.
fn require_newest(dirs: &[PathBuf], dir: &Path) -> Result<(), String> {
    match dirs.last() {
        Some(newest) if newest.as_path() == dir => Ok(()),
        newest => Err(format!(
            "{} is not the newest migration in the chain (newest: {}).\n\
             This tool only ever regenerates the newest migration — rerun it naming that one \
             instead. A shipped, mid-chain migration's down.sql, the baseline, and its frozen \
             checksum must never be regenerated from a truncated chain.",
            dir_name(dir).unwrap_or("<unknown>"),
            newest
                .and_then(|d| dir_name(d))
                .unwrap_or("<no migrations found>")
        )),
    }
}

/// True when `sql` holds no statement — only whitespace and `--` comment lines
/// (the scaffold template counts as empty).
fn is_effectively_empty(sql: &str) -> bool {
    sql.lines()
        .map(str::trim)
        .all(|line| line.is_empty() || line.starts_with("--"))
}

/// End-to-end smoke test of the tool's list-editing flow, run entirely
/// against a scratch `lib.rs` copy (`apply_list_entry` never touches the real
/// path here, so a broken test can't corrupt this checkout's source).
#[cfg(test)]
mod tests {
    use super::*;

    const SCRATCH_LIB_SRC: &str = "pub fn get_migrations() -> (MigrationSet, i32) {\n\
        let migrations = vec![\n\
            historical_migration!(\"20240120134904_init\"),\n\
            // new-migration:anchor — the tool inserts new MigrationDef entries directly above this line\n\
        ];\n\
        let count = migrations.len() as i32;\n\
        (MigrationSet { migrations }, count)\n\
    }\n";

    const UP: &str = "CREATE TABLE \"Widget\" (id INTEGER PRIMARY KEY);";
    const DOWN: &str = "DROP TABLE \"Widget\";";

    fn scratch_lib_rs() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("lib.rs");
        std::fs::write(&path, SCRATCH_LIB_SRC).unwrap();
        (dir, path)
    }

    fn dirs(names: &[&str]) -> Vec<PathBuf> {
        names.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn migration_lookup_requires_the_whole_name_not_a_suffix() {
        let all = dirs(&[
            "20260223000000_add_servers",
            "20260328000000_add_server_modloader_and_addons",
        ]);
        assert_eq!(find_migration_dir(&all, "servers"), None);
        assert_eq!(find_migration_dir(&all, "addons"), None);
        assert_eq!(
            find_migration_dir(&all, "add_servers"),
            Some(PathBuf::from("20260223000000_add_servers"))
        );
    }

    #[test]
    fn migration_lookup_picks_the_newest_when_a_name_is_reused() {
        // Scaffolding a name that already exists must operate on the directory
        // just created, not the historical one sorting before it.
        let all = dirs(&["20240120134904_init", "20260701000000_init"]);
        assert_eq!(
            find_migration_dir(&all, "init"),
            Some(PathBuf::from("20260701000000_init"))
        );
    }

    #[test]
    fn require_newest_refuses_a_mid_chain_migration_naming_the_newest() {
        let all = dirs(&[
            "20260223000000_add_servers",
            "20260328000000_add_server_modloader_and_addons",
        ]);
        let err = require_newest(&all, &all[0]).unwrap_err();
        assert!(
            err.contains("add_server_modloader_and_addons"),
            "the refusal must name the actual newest migration, got: {err}"
        );
        assert!(
            require_newest(&all, &all[1]).is_ok(),
            "the chain's newest entry must be accepted"
        );
    }

    #[test]
    fn run_refuses_a_mid_chain_migration_name_end_to_end() {
        // A shipped, mid-chain migration name reaching `run()` must be refused
        // before any generation step — regenerating the baseline or frozen
        // checksum from a truncated chain would report success while silently
        // dropping every later migration's objects.
        //
        // Only the refusal path is exercised here, deliberately: the success
        // path of `generate_or_verify` writes through `default_lib_path()` /
        // `default_checksums_test_path()` / `CARGO_MANIFEST_DIR`-rooted
        // baseline path straight into this checkout's real source files (the
        // same reason every other test in this module drives
        // `apply_list_entry` / `append_checksum_entry` against scratch paths
        // instead of calling `run()` on a real up). The refusal below returns
        // out of `run()` before any of that is reached, so it is safe to
        // exercise `run()` itself here.
        let root = tempfile::tempdir().unwrap();
        let old = root.path().join("20240101000000_add_widget");
        let newest = root.path().join("20260101000000_add_gadget");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::create_dir_all(&newest).unwrap();
        std::fs::write(old.join("migration.sql"), UP).unwrap();
        std::fs::write(
            newest.join("migration.sql"),
            "CREATE TABLE \"Gadget\" (id INTEGER PRIMARY KEY);",
        )
        .unwrap();

        let code = run(root.path(), "add_widget", false).unwrap();
        assert_eq!(
            code,
            ExitCode::FAILURE,
            "naming a mid-chain migration must refuse, not regenerate from a truncated chain"
        );
        // Nothing from the generate flow ran: no down.sql was produced.
        assert!(!old.join("down.sql").exists());
        assert!(!newest.join("down.sql").exists());
    }

    #[test]
    fn apply_list_entry_writes_the_entry_into_a_scratch_lib_rs() {
        let (_dir, lib_path) = scratch_lib_rs();
        let code = apply_list_entry(&lib_path, "20260501000000_add_widget", &[], UP, DOWN).unwrap();
        assert_eq!(code, ExitCode::SUCCESS);

        let updated = std::fs::read_to_string(&lib_path).unwrap();
        assert!(
            updated.contains("name: \"20260501000000_add_widget\","),
            "lib.rs must contain the new entry:\n{updated}"
        );
        assert!(
            updated.contains("historical_migration!(\"20240120134904_init\")"),
            "the pre-existing list content must be preserved:\n{updated}"
        );
        assert_eq!(
            updated.matches("new-migration:anchor").count(),
            1,
            "the anchor must survive the edit exactly once:\n{updated}"
        );
    }

    #[test]
    fn apply_list_entry_is_idempotent_across_reruns() {
        let (_dir, lib_path) = scratch_lib_rs();
        apply_list_entry(&lib_path, "20260501000000_add_widget", &[], UP, DOWN).unwrap();
        let once = std::fs::read_to_string(&lib_path).unwrap();

        let code = apply_list_entry(&lib_path, "20260501000000_add_widget", &[], UP, DOWN).unwrap();
        assert_eq!(code, ExitCode::SUCCESS);
        let twice = std::fs::read_to_string(&lib_path).unwrap();

        assert_eq!(
            once, twice,
            "rerunning for the same migration must not duplicate the entry"
        );
    }

    #[test]
    fn apply_list_entry_fails_loudly_when_the_anchor_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let lib_path = dir.path().join("lib.rs");
        let no_anchor = SCRATCH_LIB_SRC.replace(
            "// new-migration:anchor — the tool inserts new MigrationDef entries directly above this line\n",
            "",
        );
        std::fs::write(&lib_path, &no_anchor).unwrap();

        let code = apply_list_entry(&lib_path, "20260501000000_add_widget", &[], UP, DOWN).unwrap();
        assert_eq!(
            code,
            ExitCode::FAILURE,
            "a missing anchor must fail loudly, not silently skip"
        );
        // The file is left untouched — no partial/corrupt write.
        assert_eq!(std::fs::read_to_string(&lib_path).unwrap(), no_anchor);
    }

    const SCRATCH_CHECKSUMS_SRC: &str = "const FROZEN: &[(&str, &str)] = &[\n\
        (\n\
            \"20240120134904_init\",\n\
            \"deadbeef\",\n\
        ),\n\
        // new-migration:anchor — new_migration appends the new (name, checksum) tuple directly above this line\n\
    ];\n";

    fn scratch_checksums_test() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("migration_checksums_frozen.rs");
        std::fs::write(&path, SCRATCH_CHECKSUMS_SRC).unwrap();
        (dir, path)
    }

    #[test]
    fn append_checksum_entry_writes_the_tuple_into_a_scratch_test_file() {
        let (_dir, path) = scratch_checksums_test();
        let code = append_checksum_entry(&path, "20260501000000_add_widget", UP).unwrap();
        assert_eq!(code, ExitCode::SUCCESS);

        let updated = std::fs::read_to_string(&path).unwrap();
        assert!(
            updated.contains("\"20260501000000_add_widget\""),
            "the checksums test must contain the new migration's name:\n{updated}"
        );
        assert!(
            updated.contains(&sha256_hex(UP.as_bytes())),
            "the checksums test must contain the computed sha256 of the up SQL:\n{updated}"
        );
        assert!(
            updated.contains("\"20240120134904_init\""),
            "the pre-existing entries must be preserved:\n{updated}"
        );
        assert_eq!(
            updated.matches("new-migration:anchor").count(),
            1,
            "the anchor must survive the edit exactly once:\n{updated}"
        );
    }

    #[test]
    fn append_checksum_entry_is_idempotent_across_reruns() {
        let (_dir, path) = scratch_checksums_test();
        append_checksum_entry(&path, "20260501000000_add_widget", UP).unwrap();
        let once = std::fs::read_to_string(&path).unwrap();

        let code = append_checksum_entry(&path, "20260501000000_add_widget", UP).unwrap();
        assert_eq!(code, ExitCode::SUCCESS);
        let twice = std::fs::read_to_string(&path).unwrap();

        assert_eq!(
            once, twice,
            "rerunning for the same migration must not duplicate its checksum entry"
        );
    }

    #[test]
    fn append_checksum_entry_updates_a_stale_checksum_for_the_same_name() {
        // The still-unshipped migration's up SQL is edited between two runs of
        // the tool (`run()`'s newest-in-chain guard is what makes this safe to
        // do only for the newest, locally-authored migration — see
        // `insert_before_anchor`'s doc). Rerunning must replace the stale
        // tuple, not leave FROZEN pinned to a checksum the migration no longer
        // produces.
        let (_dir, path) = scratch_checksums_test();
        append_checksum_entry(&path, "20260501000000_add_widget", UP).unwrap();
        let stale_checksum = sha256_hex(UP.as_bytes());
        let first = std::fs::read_to_string(&path).unwrap();
        assert!(first.contains(&stale_checksum));

        const EDITED_UP: &str = "CREATE TABLE \"Widget\" (id INTEGER PRIMARY KEY, extra TEXT);";
        let new_checksum = sha256_hex(EDITED_UP.as_bytes());
        assert_ne!(
            stale_checksum, new_checksum,
            "sanity: the edit must actually change the checksum"
        );

        let code = append_checksum_entry(&path, "20260501000000_add_widget", EDITED_UP).unwrap();
        assert_eq!(code, ExitCode::SUCCESS);

        let updated = std::fs::read_to_string(&path).unwrap();
        assert!(
            updated.contains(&new_checksum),
            "the tuple must carry the new checksum:\n{updated}"
        );
        assert!(
            !updated.contains(&stale_checksum),
            "the stale checksum must be gone, not left duplicated alongside the new one:\n{updated}"
        );
        assert_eq!(
            updated.matches("20260501000000_add_widget").count(),
            1,
            "the tuple must be replaced in place, not duplicated:\n{updated}"
        );
        assert!(
            updated.contains("\"20240120134904_init\""),
            "the pre-existing, unrelated entry must be preserved:\n{updated}"
        );
        assert_eq!(
            updated.matches("new-migration:anchor").count(),
            1,
            "the anchor must survive the edit exactly once:\n{updated}"
        );
    }

    #[test]
    fn append_checksum_entry_warns_loudly_when_the_anchor_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("migration_checksums_frozen.rs");
        let no_anchor = SCRATCH_CHECKSUMS_SRC.replace(
            "// new-migration:anchor — new_migration appends the new (name, checksum) tuple directly above this line\n",
            "",
        );
        std::fs::write(&path, &no_anchor).unwrap();

        let code = append_checksum_entry(&path, "20260501000000_add_widget", UP).unwrap();
        assert_eq!(
            code,
            ExitCode::FAILURE,
            "a missing anchor must be reported as a failure, not silently skipped"
        );
        // The file is left untouched — no partial/corrupt write.
        assert_eq!(std::fs::read_to_string(&path).unwrap(), no_anchor);
    }

    #[test]
    fn combine_is_success_only_when_both_steps_succeed() {
        assert_eq!(
            combine(ExitCode::SUCCESS, ExitCode::SUCCESS),
            ExitCode::SUCCESS
        );
        assert_eq!(
            combine(ExitCode::FAILURE, ExitCode::SUCCESS),
            ExitCode::FAILURE
        );
        assert_eq!(
            combine(ExitCode::SUCCESS, ExitCode::FAILURE),
            ExitCode::FAILURE
        );
        assert_eq!(
            combine(ExitCode::FAILURE, ExitCode::FAILURE),
            ExitCode::FAILURE
        );
    }
}
