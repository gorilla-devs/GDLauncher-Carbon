//! Self-test census (spec §12 T11): the fence that fence-tests the fences.
//!
//! Every mechanical enforcement rule in the data layer — the query checker's
//! structural checks, its authorizer-driven lints, the bidirectional runner's
//! down-run verifier and refusals, and the migration generator's kind/lossiness
//! derivations — must be backed by at least one *planted-failure* self-test: a
//! test that deliberately feeds the rule a violating input and asserts the rule
//! flags it. A rule with no such test could silently regress into a no-op that
//! passes everything, and nothing would notice.
//!
//! This census makes that impossible mechanically rather than by review. Each
//! rule's implementation site carries a `// CENSUS-RULE: <id>` marker; each
//! planted-failure test carries a `// CENSUS-SELFTEST: <id>` marker. The census
//! greps both sets straight out of the source and asserts:
//!
//! 1. the set of rule markers is exactly [`EXPECTED_RULES`] (a tripwire — adding
//!    or removing an enforcement rule is a deliberate, reviewed edit here, not a
//!    silent drift);
//! 2. every rule marker is covered by at least one self-test marker (the core
//!    completeness guarantee — an uncovered rule fails the build);
//! 3. every self-test marker names a real rule (a typo or a marker left behind
//!    after a rule was removed is caught).
//!
//! Adding a new `// CENSUS-RULE:` without a matching `// CENSUS-SELFTEST:` fails
//! this test, so the census can only be satisfied by actually writing the
//! planted-failure test.
//!
//! The census's own matching logic is itself fence-tested below
//! (`census_logic_*`) with synthetic inputs, so a bug that made the census
//! vacuously pass is caught too.

use std::collections::BTreeSet;

const RULE_MARKER: &str = "CENSUS-RULE:";
const SELFTEST_MARKER: &str = "CENSUS-SELFTEST:";

/// Source files whose enforcement rules the census governs: the checker + its
/// lints, the bidirectional runner's verifier/refusals, and the migration
/// generator's derivations. (The foreign-key sweep in `fk.rs` is verified by its
/// own behavioral + fallback tests under CI T10 and its bidirectional
/// edge-list-completeness invariant; it is not part of this planted-failure
/// census.)
const RULE_SOURCE_FILES: &[&str] = &[
    "src/checker.rs",
    "src/compat.rs",
    "src/downgen.rs",
    "src/manifest.rs",
];

/// The canonical set of enforcement rules the census governs. This list is the
/// human-readable index of what the data layer mechanically enforces; it must
/// equal the set of `CENSUS-RULE` markers found in the source, so any rule
/// added or removed forces a conscious edit here.
const EXPECTED_RULES: &[&str] = &[
    // Query checker structural checks (checker.rs / check_module).
    "checker.prepare",
    "checker.declared-param-present",
    "checker.undeclared-param",
    "checker.positional-param",
    "checker.result-column-present",
    // Query checker authorizer-driven lints.
    "checker.freshness",
    "checker.nullability-nullable-source",
    "checker.nullability-expression-origin",
    "checker.query-plan-full-scan",
    "checker.handwritten-sql-registered",
    "checker.insert-datetime-explicit",
    "checker.sql-ascii-leading",
    "checker.read-class-no-writes",
    // Bidirectional runner verifier + refusals (compat.rs).
    "compat.diverged-checksum",
    "compat.backwards-missing-metadata",
    "compat.downgrade-corrupt-down",
    "compat.downgrade-schema-mismatch",
    "compat.downgrade-breaking-no-down",
    // Migration generator down derivation (downgen.rs).
    "downgen.round-trip",
    "downgen.rename-flag",
    "downgen.dml-flag",
    // Migration generator metadata derivation (manifest.rs).
    "manifest.kind-derivation",
    "manifest.data-down-undeclared-lost",
    "manifest.data-down-stale-declared",
];

/// This test file's own name — excluded when scanning `tests/` so the census
/// never reads its own marker constants or fence-test literals as real markers.
const CENSUS_FILE: &str = "self_test_census.rs";

/// Pulls every `<marker> <id>` occurrence out of `text`, returning the ids. The
/// id is the first whitespace-delimited token after the marker, so a marker in a
/// `//` comment reads exactly as one in code.
fn extract_ids(marker: &str, text: &str) -> Vec<String> {
    let mut ids = Vec::new();
    for line in text.lines() {
        if let Some(pos) = line.find(marker) {
            let rest = line[pos + marker.len()..].trim();
            if let Some(id) = rest.split_whitespace().next() {
                if !id.is_empty() {
                    ids.push(id.to_string());
                }
            }
        }
    }
    ids
}

/// Reads the governed rule-source files and returns the set of rule ids marked
/// in them.
fn source_rule_ids() -> BTreeSet<String> {
    let root = env!("CARGO_MANIFEST_DIR");
    let mut ids = BTreeSet::new();
    for rel in RULE_SOURCE_FILES {
        let path = format!("{root}/{rel}");
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("census could not read rule source {path}: {e}"));
        for id in extract_ids(RULE_MARKER, &text) {
            ids.insert(id);
        }
    }
    ids
}

/// Reads every `tests/*.rs` file except this census file and returns the set of
/// self-test rule ids marked in them.
fn selftest_ids() -> BTreeSet<String> {
    let tests_dir = format!("{}/tests", env!("CARGO_MANIFEST_DIR"));
    let mut ids = BTreeSet::new();
    for entry in std::fs::read_dir(&tests_dir)
        .unwrap_or_else(|e| panic!("census could not read {tests_dir}: {e}"))
    {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().to_string();
        if name == CENSUS_FILE || !name.ends_with(".rs") {
            continue;
        }
        let text = std::fs::read_to_string(entry.path()).unwrap();
        for id in extract_ids(SELFTEST_MARKER, &text) {
            ids.insert(id);
        }
    }
    ids
}

/// Rules present in `rules` with no self-test in `covered` — the census failure
/// set. Pure over its inputs so the census logic itself can be fence-tested.
fn uncovered_rules(rules: &BTreeSet<String>, covered: &BTreeSet<String>) -> BTreeSet<String> {
    rules.difference(covered).cloned().collect()
}

/// Self-test markers naming a rule that does not exist — a typo or a marker left
/// behind after its rule was deleted.
fn orphan_selftests(rules: &BTreeSet<String>, covered: &BTreeSet<String>) -> BTreeSet<String> {
    covered.difference(rules).cloned().collect()
}

#[test]
fn every_enforcement_rule_has_a_planted_failure_self_test() {
    let rules = source_rule_ids();
    let covered = selftest_ids();

    // A misconfigured path or a lost marker would make the census vacuously
    // pass; assert it actually found the rules it governs.
    assert!(
        !rules.is_empty(),
        "census found no CENSUS-RULE markers — the source scan is misconfigured"
    );

    // Tripwire: the discovered rule set must equal the documented canonical set.
    let expected: BTreeSet<String> = EXPECTED_RULES.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        rules,
        expected,
        "the set of CENSUS-RULE markers drifted from EXPECTED_RULES.\n\
         In source but not documented: {:?}\n\
         Documented but not in source: {:?}\n\
         Update EXPECTED_RULES (and add a planted-failure self-test for any new rule).",
        rules.difference(&expected).collect::<Vec<_>>(),
        expected.difference(&rules).collect::<Vec<_>>(),
    );

    // Core guarantee: every rule is fence-tested.
    let uncovered = uncovered_rules(&rules, &covered);
    assert!(
        uncovered.is_empty(),
        "these enforcement rules have no planted-failure self-test \
         (add a `// CENSUS-SELFTEST: <id>` test that proves the rule flags a violation): {uncovered:?}"
    );

    // Hygiene: no self-test marker may name a rule that does not exist.
    let orphans = orphan_selftests(&rules, &covered);
    assert!(
        orphans.is_empty(),
        "these CENSUS-SELFTEST markers name rules with no matching CENSUS-RULE \
         (typo, or a rule was removed without its self-test): {orphans:?}"
    );
}

// ---------------------------------------------------------------------------
// Fence-testing the census itself: the completeness logic is an enforcement
// mechanism too, so it is planted-failure-tested with synthetic inputs.
// ---------------------------------------------------------------------------

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

#[test]
fn census_logic_flags_an_uncovered_rule() {
    let rules = set(&["a.covered", "b.uncovered"]);
    let covered = set(&["a.covered"]);
    assert_eq!(uncovered_rules(&rules, &covered), set(&["b.uncovered"]));
}

#[test]
fn census_logic_passes_when_every_rule_is_covered() {
    let rules = set(&["a.one", "b.two"]);
    let covered = set(&["a.one", "b.two", "extra.but.also.an.orphan"]);
    assert!(uncovered_rules(&rules, &covered).is_empty());
}

#[test]
fn census_logic_flags_an_orphan_self_test() {
    let rules = set(&["a.one"]);
    let covered = set(&["a.one", "ghost.rule"]);
    assert_eq!(orphan_selftests(&rules, &covered), set(&["ghost.rule"]));
}

#[test]
fn census_extractor_reads_marker_ids_from_comment_lines() {
    let text = "    // CENSUS-RULE: some.rule\n\
                let x = 1; // CENSUS-SELFTEST: some.rule extra words ignored\n\
                no marker here\n";
    assert_eq!(
        extract_ids(RULE_MARKER, text),
        vec!["some.rule".to_string()]
    );
    assert_eq!(
        extract_ids(SELFTEST_MARKER, text),
        vec!["some.rule".to_string()]
    );
}
