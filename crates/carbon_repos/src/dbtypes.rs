//! Datetime codec for the on-disk format: INTEGER unix-epoch milliseconds.
//! The datetime columns store `timestamp_millis()`; every decoder that assumes
//! epoch-seconds (sqlx chrono, rusqlite's `chrono` feature) produces garbage
//! dates. All datetime traffic goes through this type.

use chrono::{DateTime, FixedOffset, NaiveDateTime, Offset, TimeZone, Utc};
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DbDateTime(pub DateTime<FixedOffset>);

pub fn to_millis(dt: &DateTime<FixedOffset>) -> i64 {
    dt.timestamp_millis()
}

pub fn from_millis(ms: i64) -> Option<DateTime<FixedOffset>> {
    Utc.timestamp_millis_opt(ms)
        .single()
        .map(|dt| dt.with_timezone(&Utc.fix()))
}

/// Decodes the TEXT datetime forms quaint accepted, for rows that predate the
/// epoch-millis codec. Migrations that backfill with SQLite's `datetime()` or
/// take a `CURRENT_TIMESTAMP` default store the first form, so these rows exist
/// in shipped databases and are not exclusively a prisma-era artifact.
fn from_text(s: &str) -> Option<DateTime<FixedOffset>> {
    let s = s.trim();

    if let Ok(naive) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f") {
        return Some(Utc.from_utc_datetime(&naive).with_timezone(&Utc.fix()));
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt);
    }
    if let Ok(dt) = DateTime::parse_from_rfc2822(s) {
        return Some(dt);
    }

    None
}

impl FromSql for DbDateTime {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value {
            ValueRef::Integer(ms) => from_millis(ms)
                .map(DbDateTime)
                .ok_or(FromSqlError::OutOfRange(ms)),
            ValueRef::Text(bytes) => std::str::from_utf8(bytes)
                .ok()
                .and_then(from_text)
                .map(DbDateTime)
                .ok_or(FromSqlError::InvalidType),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

impl ToSql for DbDateTime {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(to_millis(&self.0)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use rusqlite::Connection;

    #[test]
    fn roundtrips_epoch_millis_through_sqlite() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE t (d DATETIME)").unwrap();

        // Real value observed in production DB (spec §2.1)
        let dt = DbDateTime(from_millis(1_784_557_692_104).unwrap());
        conn.execute("INSERT INTO t (d) VALUES (?)", [&dt]).unwrap();

        // Must be stored as INTEGER storage class with the exact millis value
        let (typeof_, raw): (String, i64) = conn
            .query_row("SELECT typeof(d), d FROM t", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(typeof_, "integer");
        assert_eq!(raw, 1_784_557_692_104);

        let back: DbDateTime = conn.query_row("SELECT d FROM t", [], |r| r.get(0)).unwrap();
        assert_eq!(back.0, dt.0);
    }

    #[test]
    fn from_millis_matches_chrono() {
        let dt = from_millis(0).unwrap();
        assert_eq!(dt, Utc.timestamp_millis_opt(0).unwrap().with_timezone(&Utc.fix()));
    }

    #[test]
    fn reads_text_written_by_datetime_now_backfill() {
        // Migration 20260102000000 backfills FrontendPreference.updatedAt with
        // `datetime('now')`, which SQLite yields as TEXT, not epoch millis.
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE t (d DATETIME NOT NULL);
             INSERT INTO t (d) SELECT datetime('now');",
        )
        .unwrap();

        let stored: String = conn
            .query_row("SELECT typeof(d) FROM t", [], |r| r.get(0))
            .unwrap();
        assert_eq!(stored, "text", "precondition: the migration writes TEXT");

        let back: DbDateTime = conn.query_row("SELECT d FROM t", [], |r| r.get(0)).unwrap();
        let skew = (Utc::now().timestamp() - back.0.timestamp()).abs();
        assert!(skew < 60, "expected roughly now, got {}", back.0);
    }

    #[test]
    fn reads_text_written_by_current_timestamp_default() {
        // Migration 20240410205605 rebuilt six cache tables with
        // `DEFAULT CURRENT_TIMESTAMP` and copied rows without naming the column,
        // so every pre-existing row took the TEXT default.
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE t (d DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP);
             INSERT INTO t DEFAULT VALUES;",
        )
        .unwrap();

        let back: DbDateTime = conn.query_row("SELECT d FROM t", [], |r| r.get(0)).unwrap();
        let skew = (Utc::now().timestamp() - back.0.timestamp()).abs();
        assert!(skew < 60, "expected roughly now, got {}", back.0);
    }

    #[test]
    fn parses_the_text_formats_quaint_accepted() {
        // quaint decoded TEXT datetime columns as `%Y-%m-%d %H:%M:%S` in UTC,
        // then RFC3339, then RFC2822. Rows written under prisma may be any of them.
        let cases = [
            ("2024-04-10 20:56:05", 1_712_782_565_000i64),
            ("2024-04-10 20:56:05.123", 1_712_782_565_123),
            ("2024-04-10T20:56:05Z", 1_712_782_565_000),
            ("2024-04-10T20:56:05+00:00", 1_712_782_565_000),
            ("Wed, 10 Apr 2024 20:56:05 +0000", 1_712_782_565_000),
        ];

        for (text, expected_ms) in cases {
            let conn = Connection::open_in_memory().unwrap();
            conn.execute_batch("CREATE TABLE t (d DATETIME)").unwrap();
            conn.execute("INSERT INTO t (d) VALUES (?)", [text]).unwrap();

            let back: DbDateTime = conn.query_row("SELECT d FROM t", [], |r| r.get(0)).unwrap();
            assert_eq!(
                back.0.timestamp_millis(),
                expected_ms,
                "failed to decode {text}"
            );
        }
    }

    #[test]
    fn rejects_text_that_is_not_a_datetime() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE t (d DATETIME); INSERT INTO t VALUES ('not a date')")
            .unwrap();
        let res: Result<DbDateTime, _> = conn.query_row("SELECT d FROM t", [], |r| r.get(0));
        assert!(res.is_err(), "unparseable TEXT must still be an error");
    }
}
