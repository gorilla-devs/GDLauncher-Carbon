use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;

#[derive(FromRow, Debug)]
pub struct Skin {
    pub id: String,
    pub skin: Vec<u8>, // Using Vec<u8> to represent binary data (Bytes)
}

pub struct SkinRepository {
    pool: SqlitePool,
}

impl SkinRepository {
    pub fn new(pool: SqlitePool) -> Self {
        SkinRepository { pool }
    }

    pub async fn add_skin(&self, id: String, skin: Vec<u8>) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO skin (
                id,
                skin
            ) VALUES (?, ?)",
            id,
            skin
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_skin(&self, id: String) -> Result<Skin, sqlx::Error> {
        let skin = sqlx::query_as!(Skin, "SELECT * FROM skin WHERE id = ?", id)
            .fetch_one(&self.pool)
            .await?;

        Ok(skin)
    }
}
