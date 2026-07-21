//! Repository queries for the `Account` table.
//!
//! The account writes PCR expressed as a dynamic `Vec<SetParam>` are actually
//! driven by a fixed enum (`FullAccountType::{Offline, Microsoft}`), so each
//! variant gets its own static, checker-verified query rather than routing
//! through `DynamicQuery`.

use crate::dbtypes::DbDateTime;
use crate::queries;
use chrono::{DateTime, FixedOffset};

#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct AccountRow {
    pub uuid: String,
    pub username: String,
    pub access_token: Option<String>,
    pub token_expires: Option<DateTime<FixedOffset>>,
    pub ms_refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub gdl_token: Option<String>,
    pub last_used: DateTime<FixedOffset>,
    pub skin_id: Option<String>,
}

queries! {
    fn get_account(uuid: &str) -> Option<AccountRow> =
        "SELECT uuid, username, accessToken, tokenExpires, msRefreshToken, idToken, gdlToken, lastUsed, skinId FROM Account WHERE uuid = :uuid";
    fn get_accounts_by_last_used() -> Vec<AccountRow> =
        "SELECT uuid, username, accessToken, tokenExpires, msRefreshToken, idToken, gdlToken, lastUsed, skinId FROM Account ORDER BY lastUsed DESC";
    fn get_next_active_account(excluded_uuid: &str) -> Option<AccountRow> =
        "SELECT uuid, username, accessToken, tokenExpires, msRefreshToken, idToken, gdlToken, lastUsed, skinId FROM Account WHERE uuid <> :excluded_uuid ORDER BY lastUsed DESC LIMIT 1";

    // Offline account: only uuid/username/lastUsed are meaningful; skinId is
    // passed through for interface symmetry with `insert_account_microsoft`
    // but is always `None` at today's only call site (matching PCR, which
    // never set it for an offline `create`).
    fn insert_account_offline(uuid: &str, username: &str, last_used: DbDateTime, skin_id: Option<&str>) -> usize =
        "INSERT INTO Account (uuid, username, lastUsed, skinId) VALUES (:uuid, :username, :last_used, :skin_id)";
    fn insert_account_microsoft(
        uuid: &str,
        username: &str,
        last_used: DbDateTime,
        access_token: &str,
        token_expires: Option<DbDateTime>,
        ms_refresh_token: Option<&str>,
        id_token: Option<&str>,
        gdl_token: Option<&str>,
        skin_id: Option<&str>
    ) -> usize =
        "INSERT INTO Account (uuid, username, lastUsed, accessToken, tokenExpires, msRefreshToken, idToken, gdlToken, skinId)
         VALUES (:uuid, :username, :last_used, :access_token, :token_expires, :ms_refresh_token, :id_token, :gdl_token, :skin_id)";

    // Updating an existing account never touches `lastUsed` (see comment at
    // the call site — preserved verbatim from PCR).
    fn update_account_offline(uuid: &str, username: &str) -> usize =
        "UPDATE Account SET username = :username, accessToken = NULL, msRefreshToken = NULL, tokenExpires = NULL WHERE uuid = :uuid";
    fn update_account_microsoft(
        uuid: &str,
        username: &str,
        access_token: &str,
        token_expires: Option<DbDateTime>,
        ms_refresh_token: Option<&str>,
        id_token: Option<&str>,
        gdl_token: Option<&str>,
        skin_id: Option<&str>
    ) -> usize =
        "UPDATE Account SET username = :username, accessToken = :access_token, msRefreshToken = :ms_refresh_token,
         tokenExpires = :token_expires, idToken = :id_token, gdlToken = :gdl_token, skinId = :skin_id WHERE uuid = :uuid";

    fn set_account_gdl_token(uuid: &str, token: Option<&str>) -> usize =
        "UPDATE Account SET gdlToken = :token WHERE uuid = :uuid";
    fn expire_account_token_now(uuid: &str, now: DbDateTime) -> usize =
        "UPDATE Account SET tokenExpires = :now WHERE uuid = :uuid";
    fn update_account_profile(uuid: &str, username: &str, skin_id: Option<&str>) -> usize =
        "UPDATE Account SET username = :username, skinId = :skin_id WHERE uuid = :uuid";
    fn set_account_skin_id(uuid: &str, skin_id: Option<&str>) -> usize =
        "UPDATE Account SET skinId = :skin_id WHERE uuid = :uuid";
    fn delete_account(uuid: &str) -> usize =
        "DELETE FROM Account WHERE uuid = :uuid";
}

/// Every checkable query in this module.
pub fn all_queries() -> Vec<crate::registry::QueryCheck> {
    QUERIES.to_vec()
}
