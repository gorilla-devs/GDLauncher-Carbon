use chrono::NaiveDateTime;
use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;

#[derive(FromRow, Debug)]
pub struct LwjglMetaCache {
    pub id: String,
    pub last_updated_at: NaiveDateTime,
    pub lwjgl: Vec<u8>,
}

pub struct LwjglMetaCacheRepository {
    pool: SqlitePool,
}

impl LwjglMetaCacheRepository {
    pub fn new(pool: SqlitePool) -> Self {
        LwjglMetaCacheRepository { pool }
    }

    pub async fn add_lwjgl_meta_cache(
        &self,
        lwjgl_meta_cache: LwjglMetaCache,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO lwjgl_meta_cache (
                id,
                last_updated_at,
                lwjgl
            ) VALUES (?, ?, ?)",
            lwjgl_meta_cache.id,
            lwjgl_meta_cache.last_updated_at,
            lwjgl_meta_cache.lwjgl
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_lwjgl_meta_cache(&self, id: &str) -> Result<LwjglMetaCache, sqlx::Error> {
        let lwjgl_meta_cache = sqlx::query_as!(
            LwjglMetaCache,
            "SELECT * FROM lwjgl_meta_cache WHERE id = ?",
            id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(lwjgl_meta_cache)
    }
}
