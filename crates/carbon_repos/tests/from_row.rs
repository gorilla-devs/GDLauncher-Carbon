use carbon_repos::dbtypes::from_millis;
use carbon_repos::from_row::{FromRow, TypeClass};
use chrono::{DateTime, FixedOffset};
use rusqlite::Connection;

#[derive(carbon_macro::FromRow, Debug, PartialEq)]
struct Sample {
    id: String,
    full_version: String, // maps to column "fullVersion"
    major: i32,
    is_valid: bool, // maps to "isValid"
    #[column("type")]
    kind: String, // explicit override
    token_expires: Option<DateTime<FixedOffset>>, // "tokenExpires", epoch-ms INTEGER
    blob: Vec<u8>,
}

#[test]
fn derives_name_based_from_row_with_columns_metadata() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        r#"CREATE TABLE s (id TEXT, fullVersion TEXT, major INTEGER, isValid BOOLEAN,
                           type TEXT, tokenExpires DATETIME, blob BLOB);
           INSERT INTO s VALUES ('a', '17.0.2', 17, 1, 'Local', 1784557692104, x'0102');"#,
    )
    .unwrap();

    let got: Sample = conn
        .query_row(
            "SELECT id, fullVersion, major, isValid, type, tokenExpires, blob FROM s",
            [],
            Sample::from_row,
        )
        .unwrap();

    assert_eq!(got.id, "a");
    assert_eq!(got.full_version, "17.0.2");
    assert_eq!(got.major, 17);
    assert!(got.is_valid);
    assert_eq!(got.kind, "Local");
    assert_eq!(
        got.token_expires,
        Some(from_millis(1_784_557_692_104).unwrap())
    );
    assert_eq!(got.blob, vec![1u8, 2]);

    // COLUMNS metadata drives the checker (Task 6)
    let names: Vec<&str> = Sample::COLUMNS.iter().map(|c| c.name).collect();
    assert_eq!(
        names,
        vec![
            "id",
            "fullVersion",
            "major",
            "isValid",
            "type",
            "tokenExpires",
            "blob"
        ]
    );
    let te = Sample::COLUMNS
        .iter()
        .find(|c| c.name == "tokenExpires")
        .unwrap();
    assert!(te.nullable);
    assert!(matches!(te.ty, TypeClass::DateTime));
}
