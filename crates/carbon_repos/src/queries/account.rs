//! Account queries.

use crate::define_query;
use crate::models::{Account, Skin};

// Read queries
define_query!(FindAccountByUuid, "SELECT * FROM Account WHERE uuid = ?1", (uuid: &str) -> Account);
define_query!(ListAccounts, "SELECT * FROM Account ORDER BY lastUsed DESC", () -> Account);
define_query!(FindNextAccount, "SELECT * FROM Account WHERE uuid != ?1 ORDER BY lastUsed DESC LIMIT 1", (exclude_uuid: &str) -> Account);

// Create queries
define_query!(
    CreateAccount,
    r#"INSERT INTO Account (uuid, username, lastUsed, accessToken, msRefreshToken, tokenExpires, idToken, skinId)
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"#,
    (uuid: &str, username: &str, last_used: &str, access_token: Option<&str>, ms_refresh_token: Option<&str>, token_expires: Option<&str>, id_token: Option<&str>, skin_id: Option<&str>)
);

// Update queries
define_query!(
    UpdateAccount,
    r#"UPDATE Account SET username = ?2, accessToken = ?3, msRefreshToken = ?4, tokenExpires = ?5, idToken = ?6 WHERE uuid = ?1"#,
    (uuid: &str, username: &str, access_token: Option<&str>, ms_refresh_token: Option<&str>, token_expires: Option<&str>, id_token: Option<&str>)
);
define_query!(UpdateAccountUsername, "UPDATE Account SET username = ?2 WHERE uuid = ?1", (uuid: &str, username: &str));
define_query!(UpdateAccountTokenExpires, "UPDATE Account SET tokenExpires = ?2 WHERE uuid = ?1", (uuid: &str, token_expires: Option<&str>));
define_query!(UpdateAccountLastUsed, "UPDATE Account SET lastUsed = ?2 WHERE uuid = ?1", (uuid: &str, last_used: &str));
define_query!(UpdateAccountSkinId, "UPDATE Account SET skinId = ?2 WHERE uuid = ?1", (uuid: &str, skin_id: Option<&str>));
define_query!(
    UpdateAccountTokens,
    r#"UPDATE Account SET accessToken = ?2, msRefreshToken = ?3, tokenExpires = ?4, idToken = ?5 WHERE uuid = ?1"#,
    (uuid: &str, access_token: Option<&str>, ms_refresh_token: Option<&str>, token_expires: Option<&str>, id_token: Option<&str>)
);
define_query!(
    UpdateAccountFull,
    r#"UPDATE Account SET username = ?2, accessToken = ?3, msRefreshToken = ?4, tokenExpires = ?5, idToken = ?6, skinId = ?7 WHERE uuid = ?1"#,
    (uuid: &str, username: &str, access_token: Option<&str>, ms_refresh_token: Option<&str>, token_expires: Option<&str>, id_token: Option<&str>, skin_id: Option<&str>)
);
define_query!(
    UpdateAccountUsernameAndSkin,
    r#"UPDATE Account SET username = ?2, skinId = ?3 WHERE uuid = ?1"#,
    (uuid: &str, username: &str, skin_id: Option<&str>)
);

// Delete queries
define_query!(DeleteAccount, "DELETE FROM Account WHERE uuid = ?1", (uuid: &str));

// Skin queries
define_query!(FindSkinById, "SELECT * FROM Skin WHERE id = ?1", (id: &str) -> Skin);
define_query!(UpsertSkin, "INSERT OR REPLACE INTO Skin (id, skin) VALUES (?1, ?2)", (id: &str, skin: &[u8]));
define_query!(DeleteSkin, "DELETE FROM Skin WHERE id = ?1", (id: &str));
