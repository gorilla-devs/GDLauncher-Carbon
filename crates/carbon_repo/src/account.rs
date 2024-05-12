use chrono::NaiveDateTime;
use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;

#[derive(FromRow, Debug)]
pub struct Account {
    pub uuid: String,
    pub username: String,
    pub access_token: Option<String>,
    pub token_expires: Option<NaiveDateTime>,
    pub ms_refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub last_used: NaiveDateTime,
    pub skin_id: Option<String>,
}

pub struct AccountRepository {
    pool: SqlitePool,
}

impl AccountRepository {
    pub fn new(pool: SqlitePool) -> Self {
        AccountRepository { pool }
    }

    pub async fn add_account(&self, account: Account) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO account (
                uuid,
                username,
                access_token,
                token_expires,
                ms_refresh_token,
                id_token,
                last_used,
                skin_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            account.uuid,
            account.username,
            account.access_token,
            account.token_expires,
            account.ms_refresh_token,
            account.id_token,
            account.last_used,
            account.skin_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_account(&self, uuid: String) -> Result<Account, sqlx::Error> {
        let account = sqlx::query_as!(Account, "SELECT * FROM account WHERE uuid = ?", uuid)
            .fetch_one(&self.pool)
            .await?;

        Ok(account)
    }
}
