//! Repository queries for the `HTTPCache` table, the HTTP response cache
//! consumed by `cache_middleware`.

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
const INSERT_HTTP_CACHE_SQL: &str = "INSERT INTO HTTPCache (url, status_code, data, expiresAt, lastModified, etag)
     VALUES (:url, :status_code, :data, :expires_at, :last_modified, :etag)";

/// Replaces any cached response for `url` with a fresh one, in one
/// transaction. Mirrors the PCR `_batch` tuple (`delete_many`, `create`)
/// which relied on `_batch` only to avoid a "no rows deleted" error on the
/// first op when no cached row exists yet -- a plain `DELETE` already
/// tolerates that, so the transaction is a straight sequence of the two
/// statements.
pub fn replace_cached(
    conn: &mut rusqlite::Connection,
    url: &str,
    status_code: i32,
    data: &[u8],
    expires_at: Option<crate::dbtypes::DbDateTime>,
    last_modified: Option<&str>,
    etag: Option<&str>,
) -> Result<(), rusqlite::Error> {
    let tx = conn.transaction()?;
    tx.execute(DELETE_HTTP_CACHE_SQL, rusqlite::named_params! { ":url": url })?;
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
}

const DELETE_HTTP_CACHE_CHECK: QueryCheck = QueryCheck {
    name: "replace_cached::delete_http_cache",
    sql: DELETE_HTTP_CACHE_SQL,
    params: &[":url"],
    columns: None,
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
};

/// Every checkable query in this module: the macro-generated `QUERIES` plus
/// the two hand-written statements inside `replace_cached`.
pub fn all_queries() -> Vec<QueryCheck> {
    let mut all: Vec<QueryCheck> = QUERIES.to_vec();
    all.push(DELETE_HTTP_CACHE_CHECK);
    all.push(INSERT_HTTP_CACHE_CHECK);
    all
}
