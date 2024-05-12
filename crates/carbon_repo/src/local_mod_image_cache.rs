use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;

#[derive(FromRow, Debug)]
pub struct LocalModImageCache {
    pub metadata_id: String,
    pub data: Vec<u8>,
}

pub struct LocalModImageCacheRepository {
    pool: SqlitePool,
}

impl LocalModImageCacheRepository {
    pub fn new(pool: SqlitePool) -> Self {
        LocalModImageCacheRepository { pool }
    }

    pub async fn add_local_mod_image_cache(
        &self,
        local_mod_image_cache: LocalModImageCache,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO local_mod_image_cache (
                metadata_id,
                data
            ) VALUES (?, ?)",
            local_mod_image_cache.metadata_id,
            local_mod_image_cache.data,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_local_mod_image_cache(
        &self,
        metadata_id: &str,
    ) -> Result<LocalModImageCache, sqlx::Error> {
        let local_mod_image_cache = sqlx::query_as!(
            LocalModImageCache,
            "SELECT * FROM local_mod_image_cache WHERE metadata_id = ?",
            metadata_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(local_mod_image_cache)
    }
}
