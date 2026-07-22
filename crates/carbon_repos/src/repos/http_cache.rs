//! Repository queries for the `HTTPCache` table, the HTTP response cache
//! consumed by `cache_middleware`.

use crate::db_error::DbResult;
use crate::db_exec::{Db, WriteAccess};
use crate::dbtypes::DbDateTime;
use crate::queries;
use crate::registry::QueryCheck;
use chrono::{DateTime, FixedOffset};

#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct HttpCacheRow {
    pub url: String,
    #[column("status_code")]
    pub status_code: i32,
    pub data: Vec<u8>,
    pub expires_at: Option<DateTime<FixedOffset>>,
    pub last_modified: Option<String>,
    pub etag: Option<String>,
}

queries! {
    fn get_cached(url: &str) -> Option<HttpCacheRow> =
        "SELECT url, status_code, data, expiresAt, lastModified, etag FROM HTTPCache WHERE url = :url";
}

/// The two statements executed by `replace_cached`, kept as consts so the
/// checker validates the exact SQL the fn runs.
const DELETE_HTTP_CACHE_SQL: &str = "DELETE FROM HTTPCache WHERE url = :url";
const INSERT_HTTP_CACHE_SQL: &str =
    "INSERT INTO HTTPCache (url, status_code, data, expiresAt, lastModified, etag)
     VALUES (:url, :status_code, :data, :expires_at, :last_modified, :etag)";

/// Replaces any cached response for `url` with a fresh one, in one
/// transaction: a `DELETE` followed by an `INSERT`. The `DELETE` tolerates a
/// missing row, so no existence check is needed before it.
pub async fn replace_cached(
    db: &Db,
    url: String,
    status_code: i32,
    data: Vec<u8>,
    expires_at: Option<DbDateTime>,
    last_modified: Option<String>,
    etag: Option<String>,
) -> DbResult<()> {
    db.write(move |mut conn| {
        let tx = conn.transaction()?;
        tx.execute(
            DELETE_HTTP_CACHE_SQL,
            rusqlite::named_params! { ":url": url },
        )?;
        tx.execute(
            INSERT_HTTP_CACHE_SQL,
            rusqlite::named_params! {
                ":url": url,
                ":status_code": status_code,
                ":data": data,
                ":expires_at": expires_at,
                ":last_modified": last_modified,
                ":etag": etag,
            },
        )?;
        tx.commit()?;
        Ok(())
    })
    .await
}

const DELETE_HTTP_CACHE_CHECK: QueryCheck = QueryCheck {
    name: "replace_cached::delete_http_cache",
    sql: DELETE_HTTP_CACHE_SQL,
    params: &[":url"],
    columns: None,
    class: crate::registry::class_of(DELETE_HTTP_CACHE_SQL),
};
const INSERT_HTTP_CACHE_CHECK: QueryCheck = QueryCheck {
    name: "replace_cached::insert_http_cache",
    sql: INSERT_HTTP_CACHE_SQL,
    params: &[
        ":url",
        ":status_code",
        ":data",
        ":expires_at",
        ":last_modified",
        ":etag",
    ],
    columns: None,
    class: crate::registry::class_of(INSERT_HTTP_CACHE_SQL),
};

/// Every checkable query in this module: the macro-generated `QUERIES` plus
/// the two hand-written statements inside `replace_cached`.
pub fn all_queries() -> Vec<QueryCheck> {
    let mut all: Vec<QueryCheck> = QUERIES.to_vec();
    all.push(DELETE_HTTP_CACHE_CHECK);
    all.push(INSERT_HTTP_CACHE_CHECK);
    all
}
