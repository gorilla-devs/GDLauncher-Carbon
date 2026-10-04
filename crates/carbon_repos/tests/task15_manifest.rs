use carbon_repos::{
    compat::MigrationKind,
    downgen::{generate_down, verify_round_trip},
    manifest::{derive_kind, seeded_lost_fields, verify_data_down},
};

#[test]
fn task15_missing_primary_key_rows_are_loss() {
    for (schema, fields) in [
        (
            "CREATE TABLE Membership(id INTEGER PRIMARY KEY)",
            "Membership.id",
        ),
        (
            "CREATE TABLE Membership(a INTEGER,b INTEGER,PRIMARY KEY(a,b))",
            "Membership.a,Membership.b",
        ),
        (
            "CREATE TABLE Membership(id INTEGER PRIMARY KEY,value TEXT)",
            "Membership.id,Membership.value",
        ),
    ] {
        let up = "DELETE FROM Membership";
        verify_round_trip(&[schema], up, "").unwrap();
        assert!(
            !seeded_lost_fields(&[schema], up, "").unwrap().is_empty(),
            "missing identities: {schema}"
        );
        assert!(verify_data_down(&[schema], up, "", "full").is_err());
        verify_data_down(&[schema], up, "", &format!("partial:{fields}")).unwrap();
        verify_data_down(&[schema], "", "", "full").unwrap();
    }
}

#[test]
fn task15_set_actions_against_old_parents_are_breaking() {
    let base = "CREATE TABLE Parent(id INTEGER PRIMARY KEY)";
    for (column, action, delete_ok) in [
        ("INTEGER NOT NULL", "SET NULL", false),
        ("INTEGER NOT NULL DEFAULT 999", "SET DEFAULT", false),
        ("INTEGER CHECK(p IS NOT NULL)", "SET NULL", false),
        ("INTEGER", "SET NULL", true),
    ] {
        let up = format!("CREATE TABLE Child(p {column} REFERENCES parent(id) ON DELETE {action})");
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(&format!("PRAGMA foreign_keys=ON;{base};{up};INSERT INTO Parent VALUES(1);INSERT INTO Child VALUES(1)")).unwrap();
        assert_eq!(conn.execute("DELETE FROM Parent", []).is_ok(), delete_ok);
        assert_eq!(
            derive_kind(&[base], &up).unwrap(),
            MigrationKind::Breaking,
            "{up}"
        );
    }
}

#[test]
fn task15_same_name_index_semantics_are_breaking() {
    for (before, after, first, second) in [
        ("project,file", "project", "('p','a')", "('p','b')"),
        ("file", "lower(file)", "('p','A')", "('q','a')"),
        (
            "file COLLATE BINARY",
            "file COLLATE NOCASE",
            "('p','A')",
            "('q','a')",
        ),
        (
            "file) WHERE project='special' --",
            "file",
            "('p','a')",
            "('q','a')",
        ),
    ] {
        let base = format!(
            "CREATE TABLE Artifact(project TEXT,file TEXT); CREATE UNIQUE INDEX pair_key ON Artifact({before});\n"
        );
        let up = format!("DROP INDEX pair_key;CREATE UNIQUE INDEX pair_key ON Artifact({after});");
        assert_eq!(
            derive_kind(&[&base], &up).unwrap(),
            MigrationKind::Breaking,
            "{before} -> {after}"
        );
        let down = generate_down(&[&base], &up).unwrap();
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(&base).unwrap();
        conn.execute_batch(&up).unwrap();
        conn.execute(&format!("INSERT INTO Artifact VALUES{first}"), [])
            .unwrap();
        assert!(
            conn.execute(&format!("INSERT INTO Artifact VALUES{second}"), [])
                .is_err()
        );
        conn.execute_batch(&down).unwrap();
        conn.execute(&format!("INSERT INTO Artifact VALUES{second}"), [])
            .unwrap();
        assert_eq!(derive_kind(&[&base], "").unwrap(), MigrationKind::Additive);
    }
}

#[test]
fn task15_real_old_open_downruns_changed_unique_index_and_set_actions() {
    use carbon_repos::compat::{MigrationDef, MigrationSet, OpenVerdict};
    for (base_sql, up_sql, seed_sql, old_write) in [
        (
            "CREATE TABLE Artifact(project TEXT,file TEXT);CREATE UNIQUE INDEX pair_key ON Artifact(project,file);",
            "DROP INDEX pair_key;CREATE UNIQUE INDEX pair_key ON Artifact(project);",
            "INSERT INTO Artifact VALUES('p','a');",
            "INSERT INTO Artifact VALUES('p','b')",
        ),
        (
            "CREATE TABLE Parent(id INTEGER PRIMARY KEY);",
            "CREATE TABLE Child(p INTEGER NOT NULL REFERENCES Parent(id) ON DELETE SET NULL);",
            "INSERT INTO Parent VALUES(1);INSERT INTO Child VALUES(1);",
            "DELETE FROM Parent",
        ),
        (
            "CREATE TABLE Parent(id INTEGER PRIMARY KEY);",
            "CREATE TABLE Child(p INTEGER NOT NULL DEFAULT 999 REFERENCES Parent(id) ON DELETE SET DEFAULT);",
            "INSERT INTO Parent VALUES(1);INSERT INTO Child VALUES(1);",
            "DELETE FROM Parent",
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("db");
        let mut c = rusqlite::Connection::open(&path).unwrap();
        let base = MigrationDef {
            name: "base",
            up_sql: base_sql,
            down_sql: None,
            kind: MigrationKind::Breaking,
            data_down: "full",
        };
        let old = MigrationSet {
            migrations: vec![base],
        };
        let down = Box::leak(generate_down(&[base_sql], up_sql).unwrap().into_boxed_str());
        let new = MigrationSet {
            migrations: vec![
                base,
                MigrationDef {
                    name: "future",
                    up_sql,
                    down_sql: Some(down),
                    kind: derive_kind(&[base_sql], up_sql).unwrap(),
                    data_down: "full",
                },
            ],
        };
        new.to_latest(&mut c).unwrap();
        c.execute_batch("PRAGMA foreign_keys=ON").unwrap();
        c.execute_batch(seed_sql).unwrap();
        assert!(c.execute(old_write, []).is_err());
        assert_eq!(old.open(&mut c, &path).unwrap(), OpenVerdict::Downgraded);
        c.execute(old_write, []).unwrap();
    }
}
