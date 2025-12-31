//! Account queries.

use crate::define_query;
use crate::models::{Account, Skin};

// Read queries - typed
define_query!(
    FindAccountByUuid,
    "SELECT * FROM Account WHERE uuid = ?1",
    query_row(uuid: &str) -> Account
);
define_query!(
    ListAccounts,
    "SELECT * FROM Account ORDER BY lastUsed DESC",
    query() -> Account
);
define_query!(CountAccounts, "SELECT COUNT(*) FROM Account");
define_query!(
    FindNextAccount,
    "SELECT * FROM Account WHERE uuid != ?1 ORDER BY lastUsed DESC LIMIT 1",
    query_row(exclude_uuid: &str) -> Account
);
// Create queries
define_query!(
    CreateAccount,
    r#"INSERT INTO Account (uuid, username, lastUsed, accessToken, msRefreshToken, tokenExpires, idToken, skinId)
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"#
);

// Update queries
define_query!(
    UpdateAccount,
    r#"UPDATE Account SET
        username = ?2,
        accessToken = ?3,
        msRefreshToken = ?4,
        tokenExpires = ?5,
        idToken = ?6
    WHERE uuid = ?1"#
);

define_query!(
    UpdateAccountUsername,
    "UPDATE Account SET username = ?2 WHERE uuid = ?1"
);
define_query!(
    UpdateAccountTokenExpires,
    "UPDATE Account SET tokenExpires = ?2 WHERE uuid = ?1"
);
define_query!(
    UpdateAccountLastUsed,
    "UPDATE Account SET lastUsed = ?2 WHERE uuid = ?1"
);
define_query!(
    UpdateAccountSkinId,
    "UPDATE Account SET skinId = ?2 WHERE uuid = ?1"
);
define_query!(
    UpdateAccountTokens,
    r#"UPDATE Account SET
        accessToken = ?2,
        msRefreshToken = ?3,
        tokenExpires = ?4,
        idToken = ?5
    WHERE uuid = ?1"#
);

define_query!(
    UpdateAccountFull,
    r#"UPDATE Account SET
        username = ?2,
        accessToken = ?3,
        msRefreshToken = ?4,
        tokenExpires = ?5,
        idToken = ?6,
        skinId = ?7
    WHERE uuid = ?1"#
);

define_query!(
    UpdateAccountUsernameAndSkin,
    r#"UPDATE Account SET
        username = ?2,
        skinId = ?3
    WHERE uuid = ?1"#
);

// Delete queries - typed
define_query!(
    DeleteAccount,
    "DELETE FROM Account WHERE uuid = ?1",
    execute(uuid: &str)
);

// Skin queries - typed
define_query!(
    FindSkinById,
    "SELECT * FROM Skin WHERE id = ?1",
    query_row(id: &str) -> Skin
);
define_query!(
    UpsertSkin,
    "INSERT OR REPLACE INTO Skin (id, skin) VALUES (?1, ?2)",
    execute(id: &str, skin: &[u8])
);
define_query!(
    DeleteSkin,
    "DELETE FROM Skin WHERE id = ?1",
    execute(id: &str)
);
