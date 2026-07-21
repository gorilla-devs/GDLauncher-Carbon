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
use carbon_repos::downgen::{analyze_up, generate_down, verify_round_trip, GenError};
use carbon_repos::manifest::{derive_kind, seeded_lost_fields, DataDown};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const MIGRATIONS_SUBDIR: &str = "prisma/migrations";

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

    // A directory whose name ends in `_<name>` is this migration's home.
    let existing = dirs
        .iter()
        .find(|d| dir_name(d).map(|n| n.ends_with(&format!("_{name}"))).unwrap_or(false))
        .cloned();

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
                print_list_entry(dir_name, prev, up, &down);
                Ok(ExitCode::SUCCESS)
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
                print_list_entry(dir_name, prev, up, &down);
                Ok(ExitCode::SUCCESS)
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

/// Prints the `MigrationDef` list entry the developer pastes into
/// `get_migrations()`, with `kind` and `data_down` **derived** from the up/down
/// (spec §10.2-10.3), not left as placeholders: the same values the CI gate
/// enforces are what the tool prints, so a correct paste passes by construction.
fn print_list_entry(dir_name: &str, prev: &[&str], up: &str, down: &str) {
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

    println!("\nAdd this entry to get_migrations() in crates/carbon_repos/src/lib.rs:\n");
    println!("        MigrationDef {{");
    println!("            name: \"{dir_name}\",");
    println!(
        "            up_sql: include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/prisma/migrations/{dir_name}/migration.sql\")),"
    );
    println!(
        "            down_sql: Some(include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/prisma/migrations/{dir_name}/down.sql\"))),"
    );
    println!("            kind: {kind_expr},{kind_note}");
    println!("            data_down: \"{data_down}\",{data_note}");
    println!("        }},");
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

/// True when `sql` holds no statement — only whitespace and `--` comment lines
/// (the scaffold template counts as empty).
fn is_effectively_empty(sql: &str) -> bool {
    sql.lines()
        .map(str::trim)
        .all(|line| line.is_empty() || line.starts_with("--"))
}
