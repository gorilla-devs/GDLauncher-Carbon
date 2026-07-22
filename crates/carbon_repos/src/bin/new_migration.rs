//! `new_migration` — the migration authoring tool (spec §10), replacing
//! `prisma migrate dev`.
//!
//! Two phases, keyed on whether the migration directory's `migration.sql`
//! already holds a written `up`:
//!
//! 1. **Scaffold** — `new_migration <name>` with no matching directory creates
//!    `prisma/migrations/<utc-timestamp>_<name>/migration.sql` from an empty
//!    template. The developer writes the forward SQL there.
//! 2. **Generate** — rerun `new_migration <name>` once the `up` is written and
//!    the tool derives `down.sql` by schema diff (spec §10.1), verifies it
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

use carbon_repos::compat::MigrationKind;
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
                apply_list_entry(&default_lib_path(), dir_name, prev, up, &down)
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
                apply_list_entry(&default_lib_path(), dir_name, prev, up, &down)
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

/// Regenerates the committed fresh-install baseline (spec §11) from `prev`
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
/// and `data_down` **derived** from the up/down (spec §10.2-10.3), not left as
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
}
