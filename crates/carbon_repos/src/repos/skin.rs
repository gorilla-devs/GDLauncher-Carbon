//! Repository queries for the `Skin` table.

use crate::queries;
use crate::registry::QueryCheck;

#[derive(carbon_macro::FromRow, Debug, Clone, PartialEq)]
pub struct SkinRow {
    pub id: String,
    pub skin: Vec<u8>,
}

queries! {
    fn get_skin(id: &str) -> Option<SkinRow> =
        "SELECT id, skin FROM Skin WHERE id = :id";
}

/// The three statements executed by `replace_skin_and_link_account`, kept as
/// consts so the checker validates the exact SQL the fn runs.
const DELETE_SKIN_SQL: &str = "DELETE FROM Skin WHERE id = :id";
const INSERT_SKIN_SQL: &str = "INSERT INTO Skin (id, skin) VALUES (:id, :skin)";
const UPDATE_ACCOUNT_SKIN_SQL: &str = "UPDATE Account SET skinId = :id WHERE uuid = :uuid";

/// Replaces the cached skin `skin_id` with `skin_data` and links it to
/// `account_uuid`, in one transaction. Mirrors the PCR `_batch` tuple
/// (delete_many, create, account.update) which relied on `_batch` only to
/// avoid a "no rows deleted" error on the first op — a plain `DELETE`
/// already tolerates that, so the transaction is a straight sequence of the
/// three statements.
pub fn replace_skin_and_link_account(
    conn: &mut rusqlite::Connection,
    skin_id: &str,
    skin_data: &[u8],
    account_uuid: &str,
) -> Result<(), rusqlite::Error> {
    let tx = conn.transaction()?;
    tx.execute(DELETE_SKIN_SQL, rusqlite::named_params! { ":id": skin_id })?;
    tx.execute(
        INSERT_SKIN_SQL,
        rusqlite::named_params! { ":id": skin_id, ":skin": skin_data },
    )?;
    tx.execute(
        UPDATE_ACCOUNT_SKIN_SQL,
        rusqlite::named_params! { ":id": skin_id, ":uuid": account_uuid },
    )?;
    tx.commit()?;
    Ok(())
}

const DELETE_SKIN_CHECK: QueryCheck = QueryCheck {
    name: "replace_skin_and_link_account::delete_skin",
    sql: DELETE_SKIN_SQL,
    params: &[":id"],
    columns: None,
};
const INSERT_SKIN_CHECK: QueryCheck = QueryCheck {
    name: "replace_skin_and_link_account::insert_skin",
    sql: INSERT_SKIN_SQL,
    params: &[":id", ":skin"],
    columns: None,
};
const UPDATE_ACCOUNT_SKIN_CHECK: QueryCheck = QueryCheck {
    name: "replace_skin_and_link_account::update_account_skin",
    sql: UPDATE_ACCOUNT_SKIN_SQL,
    params: &[":id", ":uuid"],
    columns: None,
};

/// Every checkable query in this module: the macro-generated `QUERIES` plus
/// the three hand-written statements inside `replace_skin_and_link_account`.
pub fn all_queries() -> Vec<QueryCheck> {
    let mut all: Vec<QueryCheck> = QUERIES.to_vec();
    all.push(DELETE_SKIN_CHECK);
    all.push(INSERT_SKIN_CHECK);
    all.push(UPDATE_ACCOUNT_SKIN_CHECK);
    all
}
