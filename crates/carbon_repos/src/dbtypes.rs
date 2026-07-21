//! Datetime codec for the on-disk format: INTEGER unix-epoch milliseconds.
//! The datetime columns store `timestamp_millis()`; every decoder that assumes
//! epoch-seconds (sqlx chrono, rusqlite's `chrono` feature) produces garbage
//! dates. All datetime traffic goes through this type.

use chrono::{DateTime, FixedOffset, Offset, TimeZone, Utc};
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

impl FromSql for DbDateTime {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value {
            ValueRef::Integer(ms) => from_millis(ms)
                .map(DbDateTime)
                .ok_or(FromSqlError::OutOfRange(ms)),
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
    fn rejects_text_storage() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE t (d DATETIME); INSERT INTO t VALUES ('2024-01-01')")
            .unwrap();
        let res: Result<DbDateTime, _> = conn.query_row("SELECT d FROM t", [], |r| r.get(0));
        assert!(res.is_err(), "TEXT datetimes must be rejected, we only ever wrote INTEGER millis");
    }
}
